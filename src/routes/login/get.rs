use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};

/// Query parameter data
#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: Option<String>,
}

/// Login GET handler
pub async fn form(query: web::Query<QueryParams>) -> HttpResponse {
    let error_html = match query.0.error {
        None => "".to_string(),
        Some(error) => format!("<p><i>{error}</i></p>"),
    };
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
    {error_html}
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
