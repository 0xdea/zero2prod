use std::fmt::Write;

use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;

use crate::session_state::TypedSession;
use crate::utils::{err500, see_other};

/// Admin password GET handler
#[allow(clippy::future_not_send)]
pub async fn password_form(
    session: TypedSession,
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    // Validate session
    if session.get_user_id().map_err(err500)?.is_none() {
        return Ok(see_other("/login"));
    };

    // Process incoming flash messages
    let mut err_html = String::new();
    for m in flash_messages.iter() {
        writeln!(err_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    // Display password form with any error message
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(include_str!("password_form.html"), err_html)))
}
