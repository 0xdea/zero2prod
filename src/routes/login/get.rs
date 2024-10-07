use std::fmt::Write;

use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;

/// Login GET handler
pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    // Process incoming flash messages
    let mut msg = String::new();
    for m in flash_messages.iter() {
        writeln!(msg, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    // Display login form with any flash message
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(include_str!("login_form.html"), msg))
}
