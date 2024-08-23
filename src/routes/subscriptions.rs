use actix_web::{web, HttpResponse};

/// Web form data
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

/// Subscriptions handler
pub async fn subscribe(_form: web::Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
