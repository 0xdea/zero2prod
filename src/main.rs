use std::io;

use zero2prod::configuration::Settings;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), io::stdout);
    init_subscriber(subscriber);

    // Retrieve settings
    let config = Settings::get_config().expect("Failed to read configuration");

    // Build the application and run it
    let application = Application::build(config).await?;
    application.run_until_stopped().await?;
    Ok(())
}
