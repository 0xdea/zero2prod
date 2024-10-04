use actix_web::http::header::ContentType;
use actix_web::HttpResponse;

/// Admin password GET handler
pub async fn password_form() -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("password_form.html")))
}
