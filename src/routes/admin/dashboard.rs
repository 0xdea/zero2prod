use std::fmt;

use actix_web::http::header::{ContentType, LOCATION};
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::session_state::TypedSession;

// TODO: implement a login-protected admin functionality to invite more admins/collaborators

/// Admin dashboard handler
#[allow(clippy::future_not_send)]
pub async fn dashboard(
    session: TypedSession,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(err500)? {
        get_username(user_id, &db_pool).await.map_err(err500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish());
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Admin dashboard</title>
</head>
<body>
    <p>Welcome {username}!</p>
</body>
</html>"#
        )))
}

/// Return an opaque Error 500 while preserving the error's cause for logging purposes
fn err500<T>(err: T) -> actix_web::Error
where
    T: fmt::Debug + fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(err)
}

/// Retrieve the username that matches a user_id from the database
#[tracing::instrument(name = "Get Username", skip(db_pool))]
async fn get_username(user_id: Uuid, db_pool: &PgPool) -> Result<String, anyhow::Error> {
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
