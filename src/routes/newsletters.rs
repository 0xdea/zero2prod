use actix_web::HttpResponse;

/// Newsletters handler
pub async fn newsletters() -> HttpResponse {
    // TODO: Publish newsletter
    HttpResponse::Ok().finish()
}
