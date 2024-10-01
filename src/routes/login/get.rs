use std::fmt::Write;

use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::{IncomingFlashMessages, Level};

/// Login GET handler
pub async fn form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    // Process incoming flash messages
    let mut err_html = String::new();
    for m in flash_messages.iter().filter(|m| m.level() == Level::Error) {
        writeln!(err_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    // Display login form with any error message
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Login</title>
</head>
<body>
    {err_html}
    <form action="/login" method="post">
        <label>Username
            <input
                type="text"
                placeholder="Enter Username"
                name="username"
> </label>
        <label>Password
            <input
                type="password"
                placeholder="Enter Password"
                name="password"
> </label>
        <button type="submit">Login</button>
    </form>
</body>
</html>"#,
        ))
}
