use actix_web::web::ReqData;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

use crate::authentication::UserId;
use crate::domain::EmailAddress;
use crate::email_client::EmailClient;
use crate::utils::{err500, see_other};

/// Web form data
#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    content_html: String,
    content_text: String,
}

/// Confirmed subscriber data
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
    // Get the list of subscribers
    let subscribers = get_confirmed_subscribers(&db_pool).await.map_err(err500)?;

    // Send newsletter issue to each subscriber, handling errors and edge cases
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &newsletter.title,
                        &newsletter.content_html,
                        &newsletter.content_text,
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to send newsletter issue to {}",
                            subscriber.email.as_ref()
                        )
                    })
                    .map_err(err500)?;
            }
            Err(error) => {
                tracing::warn!(
                error.cause_chain = ?error,
                "Skipping a confirmed subscriber because their stored contact details are invalid",
                );
            }
        }
    }

    // Return to the endpoint and display flash message
    FlashMessage::info("The newsletter issue has been published!").send();
    Ok(see_other("/admin/newsletters"))
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
