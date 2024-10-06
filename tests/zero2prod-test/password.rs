use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::helpers::{assert_is_redirect_to, fake_password, init_test_db_pool, TestApp};

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
    let new_password = fake_password();

    let response = app
        .post_password(&serde_json::json!({
            "old_password": fake_password(),
            "new_password": &new_password,
            "new_password2": &new_password,
        }))
        .await;

    assert_is_redirect_to(&response, "/login");

    db_pool.close().await;
}

#[sqlx::test]
async fn new_password_fields_must_match(_pool_opts: PgPoolOptions, conn_opts: PgConnectOptions) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;
    let new_password = fake_password();
    let new_password2 = fake_password();

    // Login to the application
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Try to change the password
    let response = app
        .post_password(&serde_json::json!({
            "old_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password2": &new_password2,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirect
    let html = app.get_password_html().await;
    assert!(html.contains(
        "<p><i>You entered two different new passwords - \
         the field values must match</i></p>"
    ));

    db_pool.close().await;
}

#[sqlx::test]
async fn current_password_must_be_valid(_pool_opts: PgPoolOptions, conn_opts: PgConnectOptions) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;
    let old_password = fake_password();
    let new_password = fake_password();

    // Login to the application
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Try to change the password
    let response = app
        .post_password(&serde_json::json!({
            "old_password": &old_password,
            "new_password": &new_password,
            "new_password2": &new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirect
    let html = app.get_password_html().await;
    assert!(html.contains("<p><i>The current password is incorrect</i></p>"));

    db_pool.close().await;
}
