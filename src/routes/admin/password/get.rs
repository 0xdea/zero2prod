use std::fmt::Write;

use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;

/// Admin password GET handler
pub async fn password_form(
    flash_messages: IncomingFlashMessages,
) -> actix_web::Result<HttpResponse> {
    // Process incoming flash messages
    let mut msg = String::new();
    for m in flash_messages.iter() {
        writeln!(msg, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    // Display password form with any flash message
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(include_str!("password_form.html"), msg)))
}
