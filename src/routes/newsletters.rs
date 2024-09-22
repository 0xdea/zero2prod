use std::fmt;

use actix_web::http::header::{HeaderMap, HeaderValue};
use actix_web::http::{header, StatusCode};
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::engine::general_purpose;
use base64::Engine;
use secrecy::{ExposeSecret, SecretBox};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::EmailAddress;
use crate::email_client::EmailClient;
use crate::routes::helpers::error_chain_fmt;
use crate::telemetry::spawn_blocking_with_tracing;

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
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_val = HeaderValue::from_str(r#"Basic realm="newsletters""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_val);
                response
            }
            Self::UnexpectedError(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

/// Authentication credentials data
struct Credentials {
    username: String,
    password: SecretBox<String>,
}

/// Newsletters handler to send newsletter issues
#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(newsletter, db_pool, email_client, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn newsletters(
    newsletter: web::Json<NewsletterData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    // Extract authentication credentials
    let creds = basic_auth(request.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&creds.username));

    // Validate credentials and extract corresponding user_id if they are valid
    let user_id = validate_creds(creds, &db_pool).await?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

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
        r#"
        SELECT email FROM subscriptions
        WHERE status = 'confirmed'
        "#
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
        password: SecretBox::new(Box::new(password)),
    })
}

/// Validate provided authentication credentials and return user_id if they are valid
#[tracing::instrument(name = "Validate credentials", skip(creds, db_pool))]
async fn validate_creds(creds: Credentials, db_pool: &PgPool) -> Result<Uuid, PublishError> {
    // Extract stored authentication credentials for the provided username
    let (user_id, stored_password_hash) = get_stored_creds(&creds.username, db_pool)
        .await
        .map_err(PublishError::UnexpectedError)?
        .ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username")))?;

    // Verify provided password against stored password hash
    spawn_blocking_with_tracing(move || verify_password_hash(stored_password_hash, creds.password))
        .await
        .context("Failed to spawn blocking task")
        .map_err(PublishError::UnexpectedError)?
        .await?;

    Ok(user_id)
}

/// Extract stored authentication credentials from the database
#[tracing::instrument(name = "Get stored credentials", skip(username, db_pool))]
async fn get_stored_creds(
    username: &str,
    db_pool: &PgPool,
) -> Result<Option<(Uuid, SecretBox<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username
    )
    .fetch_optional(db_pool)
    .await
    .context("Failed to perform a query to validate auth credentials")?
    .map(|row| (row.user_id, SecretBox::new(Box::new(row.password_hash))));

    Ok(row)
}

/// Compare computed and stored password hashes
#[tracing::instrument(name = "Verify password hash", skip(stored_password_hash, password))]
async fn verify_password_hash(
    stored_password_hash: SecretBox<String>,
    password: SecretBox<String>,
) -> Result<(), PublishError> {
    // Parse stored password hash from PHC string format
    let stored_password_hash = PasswordHash::new(stored_password_hash.expose_secret())
        .context("Invalid stored password hash")
        .map_err(PublishError::UnexpectedError)?;

    // Compare computed and stored password hashes
    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &stored_password_hash)
        .context("Invalid password")
        .map_err(PublishError::AuthError)
}
