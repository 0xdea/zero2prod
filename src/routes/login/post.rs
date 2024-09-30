use std::fmt;

use actix_web::cookie::Cookie;
use actix_web::error::InternalError;
use actix_web::http::header::LOCATION;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use secrecy::SecretBox;
use sqlx::PgPool;

use crate::authentication::{validate_creds, AuthError, Credentials};
use crate::routes::helpers::error_chain_fmt;

/// Web form data
#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: SecretBox<String>,
}

/// Login error type
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

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }
}

/// Login POST handler
#[tracing::instrument(
    skip(form, db_pool),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    // Extract authentication credentials
    let creds = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    // Validate authentication credentials
    tracing::Span::current().record("username", tracing::field::display(&creds.username));
    match validate_creds(creds, &db_pool).await {
        // Valid credentials
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish())
        }

        // Invalid credentials
        Err(err) => {
            let err = match err {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(err.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(err.into()),
            };
            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                .cookie(Cookie::new("_flash", err.to_string()))
                .finish();
            Err(InternalError::from_response(err, response))
        }
    }
}
