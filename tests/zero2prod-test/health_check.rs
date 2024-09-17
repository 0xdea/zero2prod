use reqwest::Client;
use sqlx::PgPool;

use crate::helpers::TestApp;

#[sqlx::test]
async fn health_check_works(db_pool: PgPool) {
    let app = TestApp::spawn(db_pool).await;

    let response = Client::new()
        .get(format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}
