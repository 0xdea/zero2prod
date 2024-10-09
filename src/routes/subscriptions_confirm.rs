use std::fmt;

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::utils::error_chain_fmt;

/// Web query parameters
#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

/// Subscription confirmation error
#[derive(thiserror::Error)]
pub enum ConfirmError {
    #[error("There is no subscriber associated with the provided token")]
    UnknownToken,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl fmt::Debug for ConfirmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for ConfirmError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownToken => StatusCode::UNAUTHORIZED,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Subscription confirmation handler
/// TODO: What happens if a user clicks on a confirmation link twice?
#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, db_pool))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, ConfirmError> {
    // Get `subscriber_id` from subscription token
    let subscriber_id = get_subscriber_id_from_token(&parameters.subscription_token, &db_pool)
        .await
        .context("Failed to retrieve the subscriber id associated with the provided token")?
        .ok_or(ConfirmError::UnknownToken)?;

    // Confirm subscriber if token is valid
    confirm_subscriber(subscriber_id, &db_pool)
        .await
        .context("Failed to update subscriber status to `confirmed`")?;

    Ok(HttpResponse::Ok().finish())
}

/// Get `subscriber_id` from subscription token
/// TODO: Add validation on the incoming token, we are currently passing the raw user input straight into a query
/// TODO: Create a SubscriberId newtype
#[tracing::instrument(
    name = "Getting subscriber id from subscription token",
    skip(subscription_token, db_pool)
)]
pub async fn get_subscriber_id_from_token(
    subscription_token: &str,
    db_pool: &PgPool,
) -> sqlx::Result<Option<Uuid>> {
    let result = sqlx::query!(
        r#"
        SELECT subscriber_id
        FROM subscription_tokens
        WHERE subscription_token = $1
        "#,
        subscription_token
    )
    .fetch_optional(db_pool)
    .await?;

    Ok(result.map(|r| r.subscriber_id))
}

/// Mark subscriber as confirmed
#[tracing::instrument(name = "Marking subscriber as confirmed", skip(subscriber_id, db_pool))]
pub async fn confirm_subscriber(subscriber_id: Uuid, db_pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = 'confirmed'
        WHERE id = $1
        "#,
        subscriber_id
    )
    .execute(db_pool)
    .await?;

    Ok(())
}
