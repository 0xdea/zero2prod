use actix_web::HttpResponse;

/// Health check handler
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
