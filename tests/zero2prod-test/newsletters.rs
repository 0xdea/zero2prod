use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::{assert_is_redirect_to, init_test_db_pool, TestApp};

#[sqlx::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;

    // Create an unconfirmed subscriber for which we expect no newsletters
    app.create_unconfirmed_subscriber().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Login
    // TODO: create a login helper `app.test_user.login()`
    let body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    app.post_login(&body).await;

    // Publish the newsletter
    let body = serde_json::json!({
        "title": "Newsletter title",
        "content_text": "Newsletter body as plain text",
        "content_html": "<p>Newsletter body as HTML</p>",
    });
    let response = app.post_newsletters(&body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html = app.get_newsletters_html().await;
    assert!(html.contains("<p><i>The newsletter issue has been published!</i></p>"));

    db_pool.close().await;
}

#[sqlx::test]
async fn newsletters_are_delivered_to_confirmed_subscribers(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;

    // Create a confirmed subscriber for which we expect one newsletter
    app.create_confirmed_subscriber().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Login
    let body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    app.post_login(&body).await;

    // Publish the newsletter
    let body = serde_json::json!({
        "title": "Newsletter title",
        "content_text": "Newsletter body as plain text",
        "content_html": "<p>Newsletter body as HTML</p>",
    });
    let response = app.post_newsletters(&body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html = app.get_newsletters_html().await;
    assert!(html.contains("<p><i>The newsletter issue has been published!</i></p>"));

    db_pool.close().await;
}

#[sqlx::test]
async fn newsletters_returns_400_for_invalid_data(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;

    // Login
    let body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    app.post_login(&body).await;

    // Try to publish the newsletter with invalid data
    let test_cases = vec![
        (
            serde_json::json!({
                                "content_text": "Newsletter body as plain text",
                                "content_html": "<p>Newsletter body as HTML</p>",
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                                "title": "Newsletter!",
                                "content_html": "<p>Newsletter body as HTML</p>",
            }),
            "missing content_text",
        ),
        (
            serde_json::json!({
                                "title": "Newsletter!",
                                "content_txt": "Newsletter body as plain text",
            }),
            "missing content_html",
        ),
    ];

    #[allow(unused_variables)]
    for (body, description) in test_cases {
        let response = app.post_newsletters(&body).await;
        assert_eq!(
            response.status(),
            400,
            "The API did not fail with 400 Bad Request when the payload was {description}"
        );
    }

    db_pool.close().await;
}

#[sqlx::test]
async fn you_must_be_logged_in_to_see_the_newsletter_form(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;

    // Try to access the newsletters form
    let response = app.get_newsletters().await;
    assert_is_redirect_to(&response, "/login");

    db_pool.close().await;
}

#[sqlx::test]
async fn you_must_be_logged_in_to_publish_a_newsletter(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;

    // Try to publish the newsletter
    let body = serde_json::json!({
        "title": "Newsletter title",
        "content_text": "Newsletter body as plain text",
        "content_html": "<p>Newsletter body as HTML</p>",
    });
    let response = app.post_newsletters(&body).await;
    assert_is_redirect_to(&response, "/login");

    db_pool.close().await;
}
