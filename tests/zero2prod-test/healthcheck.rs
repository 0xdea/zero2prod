use sqlx::PgPool;

use crate::helpers::TestApp;

#[sqlx::test]
async fn healthcheck_works(db_pool: PgPool) {
    let app = TestApp::spawn(db_pool).await;

    let response = reqwest::Client::new()
        .get(format!("{}/healthcheck", &app.address))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}
