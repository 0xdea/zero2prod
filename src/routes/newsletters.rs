use std::fmt;

use actix_web::http::header::HeaderMap;
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use anyhow::Context;
use base64::engine::general_purpose;
use base64::Engine;
use secrecy::SecretBox;
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
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
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
            Self::AuthError(_) => StatusCode::UNAUTHORIZED,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Authentication credentials data
struct Credentials {
    username: String,
    password: SecretBox<String>,
}

/// Newsletters handler to send newsletter issues
pub async fn newsletters(
    newsletter: web::Json<NewsletterData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    // Extract authentication credentials
    let _creds = basic_auth(request.headers()).map_err(PublishError::AuthError)?;

    // Get the list of subscribers
    let subscribers = get_confirmed_subscribers(&db_pool).await?;

    // Send newsletter issue to each subscriber, handling errors and edge cases
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

/// Get the list of confirmed subscribers with valid email addresses
#[tracing::instrument(name = "Get confirmed subscribers", skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
        r"
        SELECT email FROM subscriptions
        WHERE status = 'confirmed'
        "
    )
    .fetch_all(db_pool)
    .await?
    .into_iter()
    .map(|row| match EmailAddress::parse(row.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();

    Ok(confirmed_subscribers)
}

/// Basic authentication credential extractor
fn basic_auth(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    // Extract the credential string from HTTP headers
    let header_val = headers
        .get("Authorization")
        .context("The 'Authorization' header was not found")?
        .to_str()
        .context("The 'Authorization' header contains invalid characters")?;
    let encoded_str = header_val
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'")?;
    let decoded_bytes = general_purpose::STANDARD
        .decode(encoded_str)
        .context("Failed to decode the credential string")?;
    let decoded_str = String::from_utf8(decoded_bytes)
        .context("The decoded credential string was not valid UTF-8")?;

    // Extract username and password from the decoded credential string
    let mut creds = decoded_str.splitn(2, ':');
    let username = creds
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth"))?
        .to_string();
    let password = creds
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth"))?
        .to_string();

    Ok(Credentials {
        username,
        password: SecretBox::from(Box::new(password)),
    })
}
