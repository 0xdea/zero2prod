#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use sqlx::postgres::PgPoolOptions;
use zero2prod::configuration::get_config;
use zero2prod::startup::Application;
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

    // Build the application and run it
    let application = Application::build(config, db_pool).await?;
    application.run_until_stopped().await?;
    Ok(())
}
