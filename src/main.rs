#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use sqlx::postgres::PgPoolOptions;
use zero2prod::configuration::get_config;
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stderr);
    init_subscriber(subscriber);

    // Get settings
    let config = get_config().expect("Failed to read configuration");

    // Connect to the database
    let db_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(config.database.db_options());

    // Build an email client
    let base_url = config.email_client.base_url().expect("Invalid base URL");
    let timeout = config.email_client.timeout();
    let sender_email = config
        .email_client
        .sender_email()
        .expect("Invalid sender email address");
    let email_client = EmailClient::new(
        base_url,
        sender_email,
        config.email_client.authorization_token,
        timeout,
    );

    run(
        std::net::TcpListener::bind(format!(
            "{}:{}",
            config.application.app_host, config.application.app_port
        ))?,
        db_pool,
        email_client,
    )?
    .await
}
