use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::helpers::{assert_is_redirect_to, init_test_db_pool, TestApp};

#[sqlx::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard(
    _pool_opts: PgPoolOptions,
    conn_opts: PgConnectOptions,
) {
    let db_pool = init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;

    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");

    db_pool.close().await;
}
