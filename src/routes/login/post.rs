use actix_web::http::header::LOCATION;
use actix_web::HttpResponse;

/// Login POST handler
pub async fn login() -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish()
}
