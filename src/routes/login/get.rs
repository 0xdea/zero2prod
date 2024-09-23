use actix_web::http::header::ContentType;
use actix_web::HttpResponse;

/// Login GET handler
pub async fn form() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("login.html"))
}
