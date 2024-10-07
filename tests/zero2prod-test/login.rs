use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::helpers::{
    assert_is_redirect_to, fake_password, fake_username, init_test_db_pool, TestApp,
};
use crate::FAKE_PASSWORD_LEN;

#[sqlx::test]
async fn an_error_flash_message_is_set_on_failure(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;

    // Try to login
    let body = serde_json::json!({
        "username": fake_username(),
        "password": fake_password(FAKE_PASSWORD_LEN),
    });
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

#[sqlx::test]
async fn redirect_to_admin_dashboard_after_login_success(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;

    // Login
    let response = app.test_user.login(&app).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Follow the redirect
    let html = app.get_dashboard_html().await;
    assert!(html.contains(&format!("Welcome {}!", app.test_user.username)));

    db_pool.close().await;
}
