use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use sha2::Sha256;

use crate::startup::HmacSecret;

/// Query parameter data
#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

// TODO: remove hmac, sha2, etc.
impl QueryParams {
    /// Verify the HMAC tag
    fn verify(self, hmac_secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(&self.tag)?;
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));

        let mut mac =
            Hmac::<Sha256>::new_from_slice(hmac_secret.0.expose_secret().as_bytes()).unwrap();

        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(self.error)
    }
}

/// Login GET handler
pub async fn form(
    query: Option<web::Query<QueryParams>>,
    hmac_secret: web::Data<HmacSecret>,
) -> HttpResponse {
    // Validate and extract error message if there are query parameters
    let err_html = query.map_or(String::new(), |query| {
        query.0.verify(&hmac_secret).map_or_else(
            |err| {
                tracing::warn!(
                    error.message = %err,
                    error.cause_chain = ?err,
                    "Failed to verify query parameters using the HMAC tag"
                );
                String::new()
            },
            |err| format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&err)),
        )
    });

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
