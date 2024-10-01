use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::helpers::{
    assert_is_redirect_to, fake_password, fake_username, init_test_db_pool, TestApp,
};

#[sqlx::test]
async fn an_error_flash_message_is_set_on_failure(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;
    let body = serde_json::json!({
        "username": fake_username(),
        "password": fake_password(),
    });

    // Try to login
    let response = app.post_login(&body).await;
    assert_is_redirect_to(&response, "/login");

    // Follow the redirect
    let html = app.get_login_html().await;
    assert!(html.contains("<p><i>Authentication failed</i></p>"));

    // Reload the login page
    let html = app.get_login_html().await;
    assert!(!html.contains("<p><i>Authentication failed</i></p>"));

    db_pool.close().await;
}
