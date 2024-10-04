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
