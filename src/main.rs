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

#[tokio::main]
async fn main() -> std::io::Result<()> {
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
