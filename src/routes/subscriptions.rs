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
pub async fn subscribe(form: web::Form<FormData>, db_pool: web::Data<PgPool>) -> HttpResponse {
    let request_id = Uuid::new_v4();
    log::info!(
        "request_id {request_id} - Adding '{}' '{}' as a new subscriber",
        form.email,
        form.name
    );

    log::info!("request_id {request_id} -Saving new subscriber details in the database");
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(db_pool.get_ref())
    .await
    .map_or_else(
        |e| {
            log::error!("request_id {request_id} -Failed to execute query: {e:?}");
            HttpResponse::InternalServerError().finish()
        },
        |_| {
            log::info!("request_id {request_id} -New subscriber details have been saved");
            HttpResponse::Ok().finish()
        },
    )
}
