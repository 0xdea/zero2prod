use std::fmt::Write;

use crate::idempotency::IdempotencyKey;
use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;

/// Newsletters GET handler
pub async fn newsletters_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    // Process incoming flash messages
    let mut msg = String::new();
    for m in flash_messages.iter() {
        writeln!(msg, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    // Display newsletters form with any flash message
    let idempotency_key = IdempotencyKey::generate();
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            include_str!("newsletters_form.html"),
            msg, idempotency_key
        ))
}
