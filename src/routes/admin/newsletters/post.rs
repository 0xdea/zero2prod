use actix_web::web::ReqData;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

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
    // Return early if we have a saved response in the database
    let user_id = user_id.into_inner();
    let FormData {
        title,
        content_html,
        content_text,
        idempotency_key,
    } = newsletter.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400_bad_request)?;
    match try_processing(&db_pool, &idempotency_key, user_id)
        .await
        .map_err(e500_internal_server_error)?
    {
        NextAction::StartProcessing => {}
        NextAction::ReturnSavedResponse(response) => {
            success_message().send();
            return Ok(response);
        }
    }

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
    let response = save_response(&db_pool, &idempotency_key, user_id, response)
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
