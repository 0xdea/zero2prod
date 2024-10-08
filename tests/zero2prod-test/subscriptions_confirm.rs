use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::TestApp;

#[sqlx::test]
async fn confirmations_without_token_are_rejected_with_a_400(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status(), 400);

    db_pool.close().await;
}

#[sqlx::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.confirmation_links(email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status(), 200);

    db_pool.close().await;
}

#[sqlx::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = TestApp::init_test_db_pool(conn_opts);
    let app = TestApp::spawn(&db_pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.confirmation_links(email_request);

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");

    db_pool.close().await;
}
