use actix_web::http::header::LOCATION;
use actix_web::HttpResponse;
use secrecy::SecretBox;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: SecretBox<String>,
}

/// Login POST handler
pub async fn login() -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish()
}
