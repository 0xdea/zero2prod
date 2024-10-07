use std::fmt;

use actix_web::error::InternalError;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::SecretBox;
use sqlx::PgPool;

use crate::authentication::{validate_creds, AuthError, Credentials};
use crate::session_state::TypedSession;
use crate::utils::{error_chain_fmt, see_other};

/// Web form
#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: SecretBox<String>,
}

/// Login error
#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl fmt::Debug for LoginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}

/// Login POST handler
#[allow(clippy::future_not_send)]
#[tracing::instrument(
    skip(form, db_pool, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, InternalError<LoginError>> {
    // Extract authentication credentials from form data
    let creds = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    // Validate authentication credentials
    tracing::Span::current().record("username", tracing::field::display(&creds.username));
    match validate_creds(creds, &db_pool).await {
        // Valid credentials: start a session and redirect to dashboard
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            // Prevent session fixation attacks
            session.renew();
            session
                .insert_user_id(user_id)
                .map_err(|e| redirect_to_login_with_error(LoginError::UnexpectedError(e.into())))?;
            Ok(see_other("/admin/dashboard"))
        }

        // Invalid credentials: return error in flash message and redirect to login
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };
            Err(redirect_to_login_with_error(e))
        }
    }
}

/// Redirect to the login page with an error message
fn redirect_to_login_with_error(err: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(err.to_string()).send();
    InternalError::from_response(err, see_other("/login"))
}
