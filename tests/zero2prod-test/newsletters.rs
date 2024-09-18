use sqlx::PgPool;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::TestApp;

#[sqlx::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers(db_pool: PgPool) {
    let app = TestApp::spawn(db_pool).await;
    app.create_unconfirmed_subscriber().await;
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

    let response = app.post_newsletters(body).await;
    assert_eq!(response.status(), 200);
}

#[sqlx::test]
async fn newsletters_are_delivered_to_confirmed_subscribers(db_pool: PgPool) {
    let app = TestApp::spawn(db_pool).await;
    app.create_confirmed_subscriber().await;
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

    let response = app.post_newsletters(body).await;
    assert_eq!(response.status(), 200);
}

#[sqlx::test]
async fn newsletters_returns_400_for_invalid_data(db_pool: PgPool) {
    let app = TestApp::spawn(db_pool).await;
    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
} }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!"}),
            "missing content",
        ),
    ];

    #[allow(unused_variables)]
    for (body, description) in test_cases {
        let response = app.post_newsletters(body).await;

        assert_eq!(
            response.status(),
            400,
            "The API did not fail with 400 Bad Request when the payload was {description}"
        );
    }
}
