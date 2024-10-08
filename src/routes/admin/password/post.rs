use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, SecretBox};
use sqlx::PgPool;

use crate::authentication::{change_password, validate_creds, AuthError, Credentials, UserId};
use crate::utils::{e303_see_other, e500_internal_server_error, get_username};

/// Web form
#[derive(serde::Deserialize)]
pub struct FormData {
    old_password: SecretBox<String>,
    new_password: SecretBox<String>,
    new_password2: SecretBox<String>,
}

/// Admin password POST handler
pub async fn password(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> actix_web::Result<HttpResponse> {
    // Validate session and retrieve associated `user_id` and `username`
    let user_id = user_id.into_inner();
    let username = get_username(*user_id, &db_pool)
        .await
        .map_err(e500_internal_server_error)?;

    // Return error in flash message and redirect back to password form if new password fields do not match
    if form.new_password.expose_secret() != form.new_password2.expose_secret() {
        FlashMessage::error("New passwords fields must match").send();
        return Ok(e303_see_other("/admin/password"));
    }

    // Return error in flash message and redirect back to password form if new password is too short
    if form.new_password.expose_secret().len() < 12 {
        FlashMessage::error("The password must be at least 12 characters long").send();
        return Ok(e303_see_other("/admin/password"));
    }

    // Return error in flash message and redirect back to password form if new password is too long
    if form.new_password.expose_secret().len() > 128 {
        FlashMessage::error("The password must contain a maximum of 128 characters").send();
        return Ok(e303_see_other("/admin/password"));
    }

    // Return error in flash message and redirect back to password form if old password is incorrect
    let creds = Credentials {
        username,
        password: form.0.old_password,
    };
    // TODO: use something similar to redirect_to_login_with_error() instead
    if let Err(e) = validate_creds(creds, &db_pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect").send();
                Ok(e303_see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500_internal_server_error(e)),
        };
    }

    // Change the password and display flash message
    change_password(*user_id, form.0.new_password, &db_pool)
        .await
        .map_err(e500_internal_server_error)?;
    FlashMessage::info("Your password has been changed").send();
    Ok(e303_see_other("/admin/password"))
}
