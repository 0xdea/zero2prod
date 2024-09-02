use crate::helpers::spawn_app;
use reqwest::Client;
use sqlx::PgPool;

#[sqlx::test]
async fn health_check_works(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    let response = Client::new()
        .get(format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
