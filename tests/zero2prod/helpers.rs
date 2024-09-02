use reqwest::Client;
use sqlx::PgPool;
use zero2prod::configuration::get_config;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

/// Ensure the tracing stack is initialized only once
static TRACING: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        init_subscriber(get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::stderr,
        ));
    } else {
        init_subscriber(get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::sink,
        ));
    };
});

/// Test application data
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        Client::new()
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }
}

/// Spin up a test application and return its data
pub async fn spawn_app(db_pool: PgPool) -> TestApp {
    // Initialize logging
    std::sync::LazyLock::force(&TRACING);

    // Get settings and modify them to use a random port for the TCP listener
    let config = {
        let mut c = get_config().expect("Failed to read configuration");
        c.application.app_port = 0;
        c
    };

    // Build the application and get its address
    let application = Application::build_with_db_pool(config, db_pool.clone())
        .await
        .expect("Failed to build application");
    let address = format!("http://127.0.0.1:{}", application.port());

    // Run the application and return its data
    #[allow(clippy::let_underscore_future)]
    let _ = tokio::spawn(application.run_until_stopped());
    TestApp { address, db_pool }
}
