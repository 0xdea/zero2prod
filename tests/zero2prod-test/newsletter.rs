use sqlx::PgPool;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::{create_confirmed_subscriber, create_unconfirmed_subscriber, spawn_app};

#[sqlx::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;
    create_unconfirmed_subscriber(&app).await;
    let body = serde_json::json!({
             "title": "Newsletter title",
             "content": {
                 "text": "Newsletter body as plain text",
                 "html": "<p>Newsletter body as HTML</p>",
             }
    });

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", &app.address))
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);
}

#[sqlx::test]
async fn newsletters_are_delivered_to_confirmed_subscribers(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;
    create_confirmed_subscriber(&app).await;
    let body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
            }
    });

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", &app.address))
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);
}
