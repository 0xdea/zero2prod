use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use wiremock::ResponseTemplate;

use crate::helpers::{when_sending_an_email, TestApp};

#[sqlx::test]
async fn subscribe_returns_a_200_for_valid_form_data(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(response.status(), 200);

    db_pool.close().await;
}

#[sqlx::test]
async fn subscribe_persists_the_new_subscriber(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let saved = sqlx::query!(
        r#"
        SELECT email, name, status
        FROM subscriptions
        "#
    )
    .fetch_one(&db_pool)
    .await
    .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");

    db_pool.close().await;
}

#[sqlx::test]
async fn subscribe_returns_a_400_when_data_is_missing(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    #[allow(unused_variables)]
    for (body, description) in test_cases {
        let response = app.post_subscriptions(body.into()).await;

        assert_eq!(
            response.status(),
            400,
            "The API did not fail with 400 Bad Request when the payload was {description}"
        );
    }

    db_pool.close().await;
}

#[sqlx::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    #[allow(unused_variables)]
    for (body, description) in test_cases {
        let response = app.post_subscriptions(body.into()).await;

        assert_eq!(
            response.status(),
            400,
            "The API did not return a 400 Bad Request when the payload was {description}"
        );
    }

    db_pool.close().await;
}

#[sqlx::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    db_pool.close().await;
}

#[sqlx::test]
async fn subscribe_sends_a_confirmation_email_with_a_link(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.confirmation_links(email_request);

    assert_eq!(confirmation_links.html, confirmation_links.text);

    db_pool.close().await;
}

#[sqlx::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    sqlx::query!(
        r#"
        ALTER TABLE subscriptions
        DROP COLUMN email;
        "#
    )
    .execute(&db_pool)
    .await
    .unwrap();

    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(response.status(), 500);

    db_pool.close().await;
}
