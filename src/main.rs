// TODO: remove temporary annoying clippy lints
#![warn(
    clippy::all,
    //clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]

use zero2prod::configuration::get_config;
use zero2prod::startup::run;

use sqlx::PgPool;
use std::net::TcpListener;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new("zero2prod".into(), std::io::stderr);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    set_global_default(subscriber).expect("Failed to set subscriber");

    // Get settings
    let config = get_config().expect("Failed to read configuration");

    // Connect to the database
    let db_pool = PgPool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to the database");

    run(
        TcpListener::bind(format!("127.0.0.1:{}", config.app_port))?,
        db_pool,
    )?
    .await
}
