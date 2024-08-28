use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// Web form data
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

/// Subscriptions handler
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, db_pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name= %form.name
    )
)]
pub async fn subscribe(form: web::Form<FormData>, db_pool: web::Data<PgPool>) -> HttpResponse {
    if !is_valid_name(&form.name) {
        return HttpResponse::BadRequest().finish();
    }
    insert_subscriber(&form, &db_pool).await.map_or_else(
        |_| HttpResponse::InternalServerError().finish(),
        |_| HttpResponse::Ok().finish(),
    )
}

/// Insert a subscriber into the database
#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, db_pool)
)]
pub async fn insert_subscriber(form: &FormData, db_pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
    })?;
    Ok(())
}

/// Validate name
pub fn is_valid_name(name: &str) -> bool {
    let is_empty = name.trim().is_empty();
    let is_too_long = name.chars().count() > 256;
    // TODO: it would be better to use a whitelist approach instead
    let blacklisted_chars = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
    let contains_blacklisted_chars = name.chars().any(|c| blacklisted_chars.contains(&c));

    !(is_empty || is_too_long || contains_blacklisted_chars)
}
