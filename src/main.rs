use std::io;

use zero2prod::configuration::Settings;
use zero2prod::delivery_worker::DeliveryWorker;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
#[allow(clippy::redundant_pub_crate)]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), io::stdout);
    init_subscriber(subscriber);

    // Retrieve settings
    let config_app = Settings::get_config().expect("Failed to load configuration");
    let config_wrk = Settings::get_config().expect("Failed to load configuration");

    // Prepare the application and the delivery worker
    let application = Application::build(config_app).await?.run_until_stopped();
    let worker = DeliveryWorker::build(config_wrk)?.run_until_stopped();

    // Run both tasks concurrently, return as soon as one of the tasks completes or errors out
    tokio::select! {
        _ = application => {},
        _ = worker => {},
    }

    Ok(())
}
