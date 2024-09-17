use std::fmt;

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use sqlx::PgPool;

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
    email: String,
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
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Newsletters handler to publish newsletters
pub async fn newsletters(
    newsletter: web::Json<NewsletterData>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, PublishError> {
    let subscribers = get_confirmed_subscribers(&db_pool).await?;
    Ok(HttpResponse::Ok().finish())
}

/// Get the list of confirmed subscribers
#[tracing::instrument(name = "Get confirmed subscribers", skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> Result<Vec<ConfirmedSubscriber>, anyhow::Error> {
    let rows = sqlx::query_as!(
        ConfirmedSubscriber,
        r"
        SELECT email FROM subscriptions
        WHERE status = 'confirmed'
        "
    )
    .fetch_all(db_pool)
    .await?;

    Ok(rows)
}
