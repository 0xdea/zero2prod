use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::helpers::{assert_is_redirect_to, fake_password, init_test_db_pool, TestApp};
use crate::FAKE_PASSWORD_LEN;

#[sqlx::test]
async fn you_must_be_logged_in_to_see_the_password_change_form(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;

    let response = app.get_password().await;
    assert_is_redirect_to(&response, "/login");

    db_pool.close().await;
}

#[sqlx::test]
async fn you_must_be_logged_in_to_change_your_password(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;
    let new_password = fake_password(FAKE_PASSWORD_LEN);

    let body = serde_json::json!({
        "old_password": fake_password(FAKE_PASSWORD_LEN),
        "new_password": &new_password,
        "new_password2": &new_password,
    });
    let response = app.post_password(&body).await;
    assert_is_redirect_to(&response, "/login");

    db_pool.close().await;
}

#[sqlx::test]
async fn new_password_fields_must_match(_pool_opts: PgPoolOptions, conn_opts: PgConnectOptions) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;
    let new_password = fake_password(FAKE_PASSWORD_LEN);
    let new_password2 = fake_password(FAKE_PASSWORD_LEN);

    // Login
    app.test_user.login(&app).await;

    // Try to change the password
    let body = serde_json::json!({
        "old_password": &app.test_user.password,
        "new_password": &new_password,
        "new_password2": &new_password2,
    });
    let response = app.post_password(&body).await;
    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirect
    let html = app.get_password_html().await;
    assert!(html.contains("<p><i>New passwords fields must match</i></p>"));

    db_pool.close().await;
}

#[sqlx::test]
async fn new_password_must_be_longer_than_12_chars(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;
    let short_password = fake_password(8);

    // Login
    app.test_user.login(&app).await;

    // Try to change the password
    let body = serde_json::json!({
        "old_password": &app.test_user.password,
        "new_password": &short_password,
        "new_password2": &short_password,
    });
    let response = app.post_password(&body).await;
    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirect
    let html = app.get_password_html().await;
    assert!(html.contains("<p><i>The password must be at least 12 characters long</i></p>"));

    db_pool.close().await;
}

#[sqlx::test]
async fn new_password_must_be_shorter_than_128_chars(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;
    let long_password = fake_password(135);

    // Login
    app.test_user.login(&app).await;

    // Try to change the password
    let body = serde_json::json!({
        "old_password": &app.test_user.password,
        "new_password": &long_password,
        "new_password2": &long_password,
    });
    let response = app.post_password(&body).await;
    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirect
    let html = app.get_password_html().await;
    assert!(html.contains("<p><i>The password must contain a maximum of 128 characters</i></p>"));

    db_pool.close().await;
}

#[sqlx::test]
async fn current_password_must_be_valid(_pool_opts: PgPoolOptions, conn_opts: PgConnectOptions) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;
    let old_password = fake_password(FAKE_PASSWORD_LEN);
    let new_password = fake_password(FAKE_PASSWORD_LEN);

    // Login
    app.test_user.login(&app).await;

    // Try to change the password
    let body = serde_json::json!({
        "old_password": &old_password,
        "new_password": &new_password,
        "new_password2": &new_password,
    });
    let response = app.post_password(&body).await;
    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirect
    let html = app.get_password_html().await;
    assert!(html.contains("<p><i>The current password is incorrect</i></p>"));

    db_pool.close().await;
}

#[sqlx::test]
async fn changing_password_works(_pool_opts: PgPoolOptions, conn_opts: PgConnectOptions) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;
    let new_password = fake_password(FAKE_PASSWORD_LEN);

    // Login
    let response = app.test_user.login(&app).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Change the password
    let body = serde_json::json!({
        "old_password": &app.test_user.password,
        "new_password": &new_password,
        "new_password2": &new_password,
    });
    let response = app.post_password(&body).await;
    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirect
    let html = app.get_password_html().await;
    assert!(html.contains("<p><i>Your password has been changed</i></p>"));

    // Logout
    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    // Follow the redirect
    let html = app.get_login_html().await;
    assert!(html.contains("<p><i>You have successfully logged out</i></p>"));

    // Login using the new password
    let body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &new_password,
    });
    let response = app.post_login(&body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    db_pool.close().await;
}
