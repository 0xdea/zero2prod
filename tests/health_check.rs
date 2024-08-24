use zero2prod::startup::run;

use sqlx::PgPool;
use std::net::TcpListener;

/// Test instance's data
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

/// Spin up a test instance and return its data
async fn spawn_app(db_pool: PgPool) -> TestApp {
    // Open a TCP listener for the web application
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");

    // Run the test instance
    let server = run(listener, db_pool.clone()).expect("Failed to bind address");
    #[allow(clippy::let_underscore_future)]
    let _ = tokio::spawn(server);

    TestApp { address, db_pool }
}

#[sqlx::test]
async fn health_check_works(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[sqlx::test]
async fn subscribe_returns_a_200_for_valid_form_data(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;
    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let response = client
        .post(format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[sqlx::test]
async fn subscribe_returns_a_400_when_data_is_missing(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, _error_message) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status(),
            "The API did not fail with 400 Bad Request when the payload was {_error_message}"
        )
    }
}
