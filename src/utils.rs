use std::{error, fmt};

use actix_web::http::header::LOCATION;
use actix_web::HttpResponse;
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};

use crate::authentication::UserId;

/// Postgres transaction type
pub type PgTransaction = Transaction<'static, Postgres>;

/// Provide a representation for any type that implements `Error`
pub fn error_chain_fmt(e: &impl error::Error, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(f, "{e}\n")?;

    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{cause}")?;
        current = cause.source();
    }

    Ok(())
}

/// Return an opaque Error 500 while preserving the error's cause for logging purposes
pub fn e500_internal_server_error<T>(e: T) -> actix_web::Error
where
    T: fmt::Debug + fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

/// Return an Error 400 with the user-representation of the validation error as body
pub fn e400_bad_request<T>(e: T) -> actix_web::Error
where
    T: fmt::Debug + fmt::Display + 'static,
{
    actix_web::error::ErrorBadRequest(e)
}

/// Return an Error 303 and redirect to the specified location
pub fn e303_see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}

/// Retrieve the username that matches a `user_id` from the database
#[tracing::instrument(name = "Get Username", skip(db_pool))]
pub async fn get_username(user_id: UserId, db_pool: &PgPool) -> anyhow::Result<String> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        *user_id
    )
    .fetch_one(db_pool)
    .await
    .context("Failed to perform a query to fetch username based on user_id")?;

    Ok(row.username)
}
