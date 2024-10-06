use actix_web::error::InternalError;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, SecretBox};
use sqlx::PgPool;
use uuid::Uuid;

use crate::authentication::{change_password, validate_creds, AuthError, Credentials};
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
    // Validate session and retrieve associated username and `user_id`
    let user_id = validate_session(&session)?;
    let username = get_username(user_id, &db_pool).await.map_err(err500)?;

    // Return error in flash message and redirect back to /admin/password if new password fields do not match
    if form.new_password.expose_secret() != form.new_password2.expose_secret() {
        FlashMessage::error("New passwords fields must match").send();
        return Ok(see_other("/admin/password"));
    }

    // Return error in flash message and redirect back to /admin/password if new password is too short
    if form.new_password.expose_secret().len() < 12 {
        FlashMessage::error("The password must be at least 12 characters long").send();
        return Ok(see_other("/admin/password"));
    }

    // Return error in flash message and redirect back to /admin/password if new password is too long
    if form.new_password.expose_secret().len() > 128 {
        FlashMessage::error("The password must contain a maximum of 128 characters").send();
        return Ok(see_other("/admin/password"));
    }

    // Return error in flash message and redirect back to /admin/password if old password is incorrect
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
            AuthError::UnexpectedError(_) => Err(err500(e)),
        };
    }

    // Change the password
    change_password(user_id, form.0.new_password, &db_pool)
        .await
        .map_err(err500)?;
    FlashMessage::info("Your password has been changed").send();
    Ok(see_other("/admin/password"))
}

/// Return `user_id` of authenticated users and reject users that are not authenticated
fn validate_session(session: &TypedSession) -> Result<Uuid, actix_web::Error> {
    session.get_user_id().map_err(err500)?.map_or_else(
        || {
            let response = see_other("/login");
            let e = anyhow::anyhow!("The user has not logged in");
            Err(InternalError::from_response(e, response).into())
        },
        Ok,
    )
}
