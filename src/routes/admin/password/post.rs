use actix_web::{web, HttpResponse};
use secrecy::SecretBox;

use crate::session_state::TypedSession;
use crate::utils::{err500, see_other};

/// Web form data
#[derive(serde::Deserialize)]
pub struct FormData {
    old_password: SecretBox<String>,
    new_password: SecretBox<String>,
    new_password2: SecretBox<String>,
}

/// Admin password POST handler
#[allow(clippy::future_not_send)]
pub async fn password(
    form: web::Form<FormData>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(err500)?.is_none() {
        return Ok(see_other("/login"));
    };
    todo!()
}
