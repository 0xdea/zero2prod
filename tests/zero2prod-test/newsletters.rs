use fake::faker::internet::en::{Password, Username};
use fake::Fake;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::{init_test_db_pool, TestApp};

#[sqlx::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(db_pool.clone()).await;
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

    db_pool.close().await;
}

#[sqlx::test]
async fn newsletters_are_delivered_to_confirmed_subscribers(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(db_pool.clone()).await;
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

    db_pool.close().await;
}

#[sqlx::test]
async fn newsletters_returns_400_for_invalid_data(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(db_pool.clone()).await;
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

    db_pool.close().await;
}

#[sqlx::test]
async fn requests_missing_authorization_are_rejected(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(db_pool.clone()).await;

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", app.address))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="newsletters""#
    );

    db_pool.close().await;
}

#[sqlx::test]
async fn non_existing_user_is_rejected(_pool_opts: PgPoolOptions, conn_opts: PgConnectOptions) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(db_pool.clone()).await;

    // Invalid username and password
    let username: String = Username().fake();
    let password: String = Password(32..33).fake();

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", app.address))
        .basic_auth(username, Some(password))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="newsletters""#
    );

    db_pool.close().await;
}

#[sqlx::test]
async fn invalid_password_is_rejected(_pool_opts: PgPoolOptions, conn_opts: PgConnectOptions) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(db_pool.clone()).await;

    // Valid username and invalid password
    let username = &app.test_user.username;
    let password: String = Password(32..33).fake();
    assert_ne!(app.test_user.password, password);

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", app.address))
        .basic_auth(username, Some(password))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="newsletters""#
    );

    db_pool.close().await;
}
