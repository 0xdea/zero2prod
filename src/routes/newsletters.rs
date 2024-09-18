use std::fmt;

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;

use crate::domain::EmailAddress;
use crate::email_client::EmailClient;
use crate::routes::helpers::error_chain_fmt;

/// Newsletter data
#[derive(serde::Deserialize)]
pub struct NewsletterData {
    title: String,
    content: Content,
}

/// Newsletter content
#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

/// Confirmed subscriber data
struct ConfirmedSubscriber {
    email: EmailAddress,
}

/// Publish error type
#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl fmt::Debug for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Newsletters handler to send newsletter issues
pub async fn newsletters(
    newsletter: web::Json<NewsletterData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
    // Get the list of subscribers
    let subscribers = get_confirmed_subscribers(&db_pool).await?;

    // Send the newsletter issue to each subscriber
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &newsletter.title,
                        &newsletter.content.html,
                        &newsletter.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to send newsletter issue to {}",
                            subscriber.email.as_ref()
                        )
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                error.cause_chain = ?error,
                "Skipping a confirmed subscriber because their stored contact details are invalid",
                );
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

/// Get the list of confirmed subscribers
#[tracing::instrument(name = "Get confirmed subscribers", skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    // Row type to map the data coming out of the SQL query below
    struct Row {
        email: String,
    }

    // Query the database
    let rows = sqlx::query_as!(
        Row,
        r"
        SELECT email FROM subscriptions
        WHERE status = 'confirmed'
        "
    )
    .fetch_all(db_pool)
    .await?;

    // Map into the domain type, handling the edge case in which a subscriber has an invalid email address
    let confirmed_subscribers = rows
        .into_iter()
        .map(|row| match EmailAddress::parse(row.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();

    Ok(confirmed_subscribers)
}
