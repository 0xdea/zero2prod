use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::helpers::TestApp;

#[sqlx::test]
async fn healthcheck_works(_pool_opts: PgPoolOptions, conn_opts: PgConnectOptions) {
    let db_pool = TestApp::init_test_db_pool(conn_opts).await;
    let app = TestApp::spawn(&db_pool).await;

    let response = reqwest::Client::new()
        .get(format!("{}/healthcheck", &app.address))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));

    db_pool.close().await;
}
