// TODO: remove temporary annoying clippy lints
#![warn(
    clippy::all,
    //clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]

use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use zero2prod::configuration::get_config;
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
        .connect_lazy(config.database.database_url().expose_secret())
        .expect("Failed to connect to the database");

    run(
        std::net::TcpListener::bind(format!(
            "{}:{}",
            config.application.app_host, config.application.app_port
        ))?,
        db_pool,
    )?
    .await
}
