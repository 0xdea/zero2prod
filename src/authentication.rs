use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, SecretBox};
use sqlx::PgPool;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_tracing;

/// Fallback hash in case an invalid username is provided during authentication
const FALLBACK_HASH: &str =
    "$argon2id$v=19$m=15000,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno";

/// Authentication credentials data
pub struct Credentials {
    pub username: String,
    pub password: SecretBox<String>,
}

/// Authentication error type
#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

/// Validate provided authentication credentials and return user_id if they are valid
#[tracing::instrument(name = "Validate credentials", skip(creds, db_pool))]
pub async fn validate_creds(creds: Credentials, db_pool: &PgPool) -> Result<Uuid, AuthError> {
    // Fallback user_id and password hash to prevent timing attacks
    let mut user_id = None;
    let mut expected_password_hash = SecretBox::new(Box::new(FALLBACK_HASH.to_string()));

    // Extract stored authentication credentials for the provided username
    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_creds(&creds.username, db_pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    // Verify provided password against stored password hash
    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, creds.password)
    })
    .await
    .context("Failed to spawn blocking task")?
    .await?;

    user_id.ok_or_else(|| AuthError::InvalidCredentials(anyhow::anyhow!("Unknown username")))
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
    .map(|r| (r.user_id, SecretBox::new(Box::new(r.password_hash))));

    Ok(row)
}

/// Compare computed and stored password hashes
#[tracing::instrument(name = "Verify password hash", skip(password_hash, password))]
async fn verify_password_hash(
    password_hash: SecretBox<String>,
    password: SecretBox<String>,
) -> Result<(), AuthError> {
    // Parse stored password hash from PHC string format
    let password_hash =
        PasswordHash::new(password_hash.expose_secret()).context("Invalid stored password hash")?;

    // Compare computed and stored password hashes
    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &password_hash)
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}
