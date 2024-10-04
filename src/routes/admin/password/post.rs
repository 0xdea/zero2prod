use actix_web::{web, HttpResponse};
use secrecy::SecretBox;

/// Web form data
#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: SecretBox<String>,
    new_password: SecretBox<String>,
    new_password2: SecretBox<String>,
}

/// Admin password POST handler
pub async fn password(_form: web::Form<FormData>) -> Result<HttpResponse, actix_web::Error> {
    todo!()
}
