use std::fmt;

use actix_web::http::header::LOCATION;
use actix_web::HttpResponse;
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

/// Return an opaque Error 500 while preserving the error's cause for logging purposes
pub fn err500<T>(err: T) -> actix_web::Error
where
    T: fmt::Debug + fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(err)
}

/// Return an Error 303 and redirect to the specified location
pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}

/// Retrieve the username that matches a `user_id` from the database
#[tracing::instrument(name = "Get Username", skip(db_pool))]
pub async fn get_username(user_id: Uuid, db_pool: &PgPool) -> anyhow::Result<String> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(db_pool)
    .await
    .context("Failed to perform a query to fetch username based on user_id")?;

    Ok(row.username)
}
