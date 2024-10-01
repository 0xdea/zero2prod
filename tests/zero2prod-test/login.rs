use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::helpers::{assert_is_redirect_to, init_test_db_pool, TestApp};

#[sqlx::test]
async fn an_error_flash_message_is_set_on_failure(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;
    let body = serde_json::json!({
        "username": "random_username",
        "password": "random_password",
    });

    // Try to login and follow redirect
    let response = app.post_login(&body).await;
    assert_is_redirect_to(&response, "/login");

    // Check flash message in cookie
    let flash_cookie = response.cookies().find(|c| c.name() == "_flash").unwrap();
    assert_eq!(flash_cookie.value(), "Authentication failed");

    // Follow the redirect
    let html = app.get_login_html().await;
    assert!(html.contains(r"<p><i>Authentication failed</i></p>"));

    // Reload the login page
    let html = app.get_login_html().await;
    assert!(!html.contains(r"<p><i>Authentication failed</i></p>"));

    db_pool.close().await;
}
