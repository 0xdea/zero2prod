use std::fmt;
use std::ops::Deref;

use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::authentication::UserId;
use crate::idempotency::{save_response, try_processing, IdempotencyKey, NextAction};
use crate::utils::{e303_see_other, e400_bad_request, e500_internal_server_error};

/// Web form
#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    content_html: String,
    content_text: String,
    idempotency_key: String,
}

/// Newsletters handler
#[allow(clippy::future_not_send)]
#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn newsletters(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> actix_web::Result<HttpResponse> {
    // Return early if we have a saved response in the database, otherwise start processing the request
    let user_id = user_id.into_inner();
    let FormData {
        title,
        content_html,
        content_text,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400_bad_request)?;
    let mut transaction = match try_processing(&db_pool, &idempotency_key, user_id)
        .await
        .map_err(e500_internal_server_error)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(response) => {
            success_message().send();
            return Ok(response);
        }
    };

    // Store newsletter issue in the database and create a task in the issue delivery queue
    let issue_id = insert_newsletter_issue(&mut transaction, &title, &content_html, &content_text)
        .await
        .context("Failed to store newsletter issue in the database")
        .map_err(e500_internal_server_error)?;
    enqueue_delivery_task(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery task")
        .map_err(e500_internal_server_error)?;

    // Save response for idempotency, redirect back to the endpoint, and display flash message
    let response = e303_see_other("/admin/newsletters");
    let response = save_response(transaction, &idempotency_key, user_id, response)
        .await
        .map_err(e500_internal_server_error)?;
    success_message().send();
    Ok(response)
}

/// Return a flash message in case of successful newsletter publication
fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been accepted, emails will go out shortly")
}

/// Newsletter issue identifier
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NewsletterIssueId(Uuid);

impl NewsletterIssueId {
    pub const fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl fmt::Display for NewsletterIssueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for NewsletterIssueId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Store newsletter issue in the database
#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    content_html: &str,
    content_text: &str,
) -> sqlx::Result<NewsletterIssueId> {
    // Save newsletter issue to the database
    let newsletter_issue_id = NewsletterIssueId::new(Uuid::new_v4());
    transaction
        .execute(sqlx::query!(
            r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            content_html,
            content_text,
            published_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
            *newsletter_issue_id,
            title,
            content_html,
            content_text,
        ))
        .await?;

    // Return newsletter id
    Ok(newsletter_issue_id)
}

/// Create a task in the issue delivery queue
#[tracing::instrument(skip_all)]
async fn enqueue_delivery_task(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: NewsletterIssueId,
) -> sqlx::Result<()> {
    // Create a task in issue delivery queue table stored in the database
    transaction
        .execute(sqlx::query!(
            r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
            *newsletter_issue_id,
        ))
        .await?;

    Ok(())
}
