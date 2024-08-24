use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use tracing::Instrument;
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
    let request_span = tracing::info_span!(
        "Adding a new subscriber",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name
    );
    let _request_span_guard = request_span.enter();

    let query_span = tracing::info_span!("Saving new subscriber details in the database");
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(db_pool.as_ref())
    .instrument(query_span)
    .await
    .map_or_else(
        |e| {
            tracing::error!("request_id {request_id} - Failed to execute query: {e:?}");
            HttpResponse::InternalServerError().finish()
        },
        |_| {
            tracing::info!("request_id {request_id} - New subscriber details have been saved");
            HttpResponse::Ok().finish()
        },
    )
}
