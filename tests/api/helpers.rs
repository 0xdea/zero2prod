use sqlx::PgPool;
use zero2prod::configuration::get_config;
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
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

/// Test instance data
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

/// Spin up a test instance and return its data
pub async fn spawn_app(db_pool: PgPool) -> TestApp {
    // Initialize logging
    std::sync::LazyLock::force(&TRACING);

    // Open a TCP listener for the web application
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");

    // Get settings
    let config = get_config().expect("Failed to read configuration");

    // Build a new email client
    let base_url = config.email_client.base_url().expect("Invalid base URL");
    let sender_email = config
        .email_client
        .sender_email()
        .expect("Invalid sender email address.");
    let email_client = EmailClient::new(
        base_url,
        sender_email,
        config.email_client.authorization_token,
        std::time::Duration::from_millis(200),
    );

    // Run the test instance
    let server = run(listener, db_pool.clone(), email_client).expect("Failed to bind address");
    #[allow(clippy::let_underscore_future)]
    let _ = tokio::spawn(server);

    TestApp { address, db_pool }
}
