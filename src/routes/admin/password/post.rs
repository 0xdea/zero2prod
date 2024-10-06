use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, SecretBox};
use sqlx::PgPool;

use crate::authentication::{validate_creds, AuthError, Credentials};
use crate::session_state::TypedSession;
use crate::utils::{err500, get_username, see_other};

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
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    // Validate session and retrieve associated username
    let username = if let Some(user_id) = session.get_user_id().map_err(err500)? {
        get_username(user_id, &db_pool).await.map_err(err500)?
    } else {
        return Ok(see_other("/login"));
    };

    // Return error in flash message and redirect to /admin/password if new password fields do not match
    if form.new_password.expose_secret() != form.new_password2.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }

    let creds = Credentials {
        username,
        password: form.0.old_password,
    };
    // TODO: use something similar to redirect_to_login_with_error() instead
    if let Err(e) = validate_creds(creds, &db_pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(err500(e).into()),
        };
    }
    todo!()
}
