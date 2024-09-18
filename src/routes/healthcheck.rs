use actix_web::HttpResponse;

/// Health check handler
pub async fn healthcheck() -> HttpResponse {
    HttpResponse::Ok().finish()
}
