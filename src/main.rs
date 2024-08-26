// TODO: remove temporary annoying clippy lints
#![warn(
    clippy::all,
    //clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]

use std::net::TcpListener;

use secrecy::ExposeSecret;
use sqlx::PgPool;

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
    let db_pool = PgPool::connect(&config.database.database_url().expose_secret())
        .await
        .expect("Failed to connect to the database");

    run(
        TcpListener::bind(format!("127.0.0.1:{}", config.app_port))?,
        db_pool,
    )?
    .await
}
