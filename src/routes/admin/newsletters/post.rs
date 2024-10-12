use std::fmt;
use std::ops::Deref;

use actix_web::web::ReqData;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::authentication::UserId;
use crate::domain::EmailAddress;
use crate::email_client::EmailClient;
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

/// Confirmed subscriber
struct ConfirmedSubscriber {
    email: EmailAddress,
}

/// Newsletters handler
//noinspection RsLiveness
#[allow(clippy::future_not_send)]
#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(newsletter, db_pool, email_client, user_id),
    fields(user_id=%*user_id)
)]
pub async fn newsletters(
    newsletter: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: ReqData<UserId>,
) -> actix_web::Result<HttpResponse> {
    // Return early if we have a saved response in the database, otherwise start processing the request
    let user_id = user_id.into_inner();
    let FormData {
        title,
        content_html,
        content_text,
        idempotency_key,
    } = newsletter.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400_bad_request)?;
    let transaction = match try_processing(&db_pool, &idempotency_key, user_id)
        .await
        .map_err(e500_internal_server_error)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(response) => {
            success_message().send();
            return Ok(response);
        }
    };

    // Get the list of confirmed subscribers
    let subscribers = get_confirmed_subscribers(&db_pool)
        .await
        .map_err(e500_internal_server_error)?;

    // Send a newsletter issue to each subscriber, handling errors and edge cases
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &title, &content_html, &content_text)
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })
                    .map_err(e500_internal_server_error)?;
            }
            Err(error) => {
                tracing::warn!(
                error.cause_chain = ?error,
                "Skipping a confirmed subscriber because their stored contact details are invalid",
                );
            }
        }
    }

    // Save response for idempotency, redirect back to the endpoint, and display flash message
    success_message().send();
    let response = e303_see_other("/admin/newsletters");
    let response = save_response(transaction, &idempotency_key, user_id, response)
        .await
        .map_err(e500_internal_server_error)?;
    Ok(response)
}

/// Get the list of confirmed subscribers with valid email addresses
#[tracing::instrument(name = "Get confirmed subscribers", skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> anyhow::Result<Vec<anyhow::Result<ConfirmedSubscriber>>> {
    let confirmed_subscribers = sqlx::query!(
        r#"
        SELECT email FROM subscriptions
        WHERE status = 'confirmed'
        "#
    )
    .fetch_all(db_pool)
    .await?
    .into_iter()
    .map(|r| match EmailAddress::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();

    Ok(confirmed_subscribers)
}

/// Return a flash message in case of successful newsletter publication
fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been published!")
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

/// Save newsletter issue to the database
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
