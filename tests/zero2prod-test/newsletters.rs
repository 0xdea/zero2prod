use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use wiremock::ResponseTemplate;

use zero2prod::idempotency::IdempotencyKey;

use crate::helpers::{assert_is_redirect_to, when_sending_an_email, TestApp};

#[sqlx::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;

    // Create an unconfirmed subscriber for which we expect no newsletters
    app.create_unconfirmed_subscriber().await;
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Login
    app.test_user.login(&app).await;

    // Publish the newsletter
    let body = serde_json::json!({
        "title": "Newsletter title",
        "content_text": "Newsletter body as plain text",
        "content_html": "<p>Newsletter body as HTML</p>",
        "idempotency_key": IdempotencyKey::generate(),
    });
    let response = app.post_newsletters(&body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html = app.get_newsletters_html().await;
    assert!(html.contains(
        "<p><i>The newsletter issue has been accepted, emails will go out shortly</i></p>"
    ));

    // Consume all enqueued tasks
    app.dispatch_all_pending_emails(&db_pool).await;

    db_pool.close().await;
}

#[sqlx::test]
async fn newsletters_are_delivered_to_confirmed_subscribers(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;

    // Create a confirmed subscriber for which we expect one newsletter
    app.create_confirmed_subscriber().await;
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Login
    app.test_user.login(&app).await;

    // Publish the newsletter
    let body = serde_json::json!({
        "title": "Newsletter title",
        "content_text": "Newsletter body as plain text",
        "content_html": "<p>Newsletter body as HTML</p>",
        "idempotency_key": IdempotencyKey::generate()
    });
    let response = app.post_newsletters(&body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html = app.get_newsletters_html().await;
    assert!(html.contains(
        "<p><i>The newsletter issue has been accepted, emails will go out shortly</i></p>"
    ));

    // Consume all enqueued tasks
    app.dispatch_all_pending_emails(&db_pool).await;

    db_pool.close().await;
}

#[sqlx::test]
async fn newsletters_returns_400_for_invalid_data(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;

    // Login
    app.test_user.login(&app).await;

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
    let db_pool = TestApp::init_test_db_pool(conn_opts);
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
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;

    // Try to publish the newsletter
    let body = serde_json::json!({
        "title": "Newsletter title",
        "content_text": "Newsletter body as plain text",
        "content_html": "<p>Newsletter body as HTML</p>",
        "idempotency_key": IdempotencyKey::generate()
    });
    let response = app.post_newsletters(&body).await;
    assert_is_redirect_to(&response, "/login");

    db_pool.close().await;
}

#[sqlx::test]
async fn newsletter_creation_is_idempotent(_pool_opts: PgPoolOptions, conn_opts: PgConnectOptions) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;

    // Create a confirmed subscriber for which we expect only one newsletter
    app.create_confirmed_subscriber().await;
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Login
    app.test_user.login(&app).await;

    // Publish the newsletter
    let body = serde_json::json!({
        "title": "Newsletter title",
        "content_text": "Newsletter body as plain text",
        "content_html": "<p>Newsletter body as HTML</p>",
        "idempotency_key": IdempotencyKey::generate()
    });
    let response = app.post_newsletters(&body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html = app.get_newsletters_html().await;
    assert!(html.contains(
        "<p><i>The newsletter issue has been accepted, emails will go out shortly</i></p>"
    ));

    // Try to publish the newsletter again
    let response = app.post_newsletters(&body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html = app.get_newsletters_html().await;
    assert!(html.contains(
        "<p><i>The newsletter issue has been accepted, emails will go out shortly</i></p>"
    ));

    // Consume all enqueued tasks
    app.dispatch_all_pending_emails(&db_pool).await;

    db_pool.close().await;
}

#[sqlx::test]
async fn concurrent_form_submission_is_handled_gracefully(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;

    // Create a confirmed subscriber for which we expect only one newsletter
    app.create_confirmed_subscriber().await;
    // Set a long delay to ensure that the second request arrives before the first one completes
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Login
    app.test_user.login(&app).await;

    // Submit two newsletter forms concurrently
    let body = serde_json::json!({
        "title": "Newsletter title",
        "content_text": "Newsletter body as plain text",
        "content_html": "<p>Newsletter body as HTML</p>",
        "idempotency_key": IdempotencyKey::generate()
    });
    let response1 = app.post_newsletters(&body);
    let response2 = app.post_newsletters(&body);
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );

    // Consume all enqueued tasks
    app.dispatch_all_pending_emails(&db_pool).await;

    db_pool.close().await;
}
