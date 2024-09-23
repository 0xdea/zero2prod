use actix_web::HttpResponse;

/// Login POST handler
pub async fn login() -> HttpResponse {
    HttpResponse::Ok().finish()
}
