use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, SecretBox};

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
    // Validate session
    if session.get_user_id().map_err(err500)?.is_none() {
        return Ok(see_other("/login"));
    };

    // Return error in flash message and redirect to /admin/password if new password fields do not match
    if form.new_password.expose_secret() != form.new_password2.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }
    todo!()
}
