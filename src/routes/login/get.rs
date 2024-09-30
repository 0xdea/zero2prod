use actix_web::http::header::ContentType;
use actix_web::{HttpRequest, HttpResponse};

/// Login GET handler
#[allow(clippy::future_not_send)]
pub async fn form(request: HttpRequest) -> HttpResponse {
    // Extract error message from cookie
    let err_html = request
        .cookie("_flash")
        .map_or(String::new(), |c| format!("<p><i>{}</i></p>", c.value()));

    // Display login form
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
