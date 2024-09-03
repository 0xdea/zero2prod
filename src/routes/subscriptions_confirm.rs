use actix_web::{web, HttpResponse};

/// Web query parameters
#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

/// Confirmation handler
#[tracing::instrument(name = "Confirm a pending subscriber", skip(_parameters))]
pub async fn confirm(_parameters: web::Query<Parameters>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
