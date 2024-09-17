use actix_web::{web, HttpResponse};

/// Newsletter data
#[derive(serde::Deserialize)]
pub struct NewsletterData {
    title: String,
    content: Content,
}

/// Newsletter content
#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

/// Newsletters handler to publish newsletters
pub async fn newsletters(_newsletter: web::Json<NewsletterData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
