use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::helpers::{assert_is_redirect_to, init_test_db_pool, TestApp};

#[sqlx::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;

    let response = app.get_dashboard().await;
    assert_is_redirect_to(&response, "/login");

    db_pool.close().await;
}

#[sqlx::test]
async fn logout_clears_session_state(_pool_opts: PgPoolOptions, conn_opts: PgConnectOptions) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;

    // Login
    let response = app.test_user.login(&app).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Logout
    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    // Follow the redirect
    let html = app.get_login_html().await;
    assert!(html.contains("<p><i>You have successfully logged out</i></p>"));

    // Attempt to access admin dashboard after logout
    let response = app.get_dashboard().await;
    assert_is_redirect_to(&response, "/login");

    db_pool.close().await;
}
