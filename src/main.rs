use std::fmt::{Debug, Display};
use std::io;

use tokio::task::JoinError;

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
    let config = Settings::get_config().expect("Failed to load configuration");

    // Prepare the application and the delivery worker tasks
    let task_app = tokio::spawn(
        Application::build(config.clone())
            .await?
            .run_until_stopped(),
    );
    let task_wrk = tokio::spawn(DeliveryWorker::build(config)?.run_until_stopped());

    // Run both tasks concurrently
    tokio::select! {
        o = task_app => report_exit("Application", o),
        o = task_wrk => report_exit("Delivery Worker", o),
    }
    Ok(())
}

/// Report info or error on task exit
fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        // Task exited
        Ok(Ok(())) => {
            tracing::info!("{task_name} has exited");
        }

        // Task failed
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{task_name} failed"
            );
        }

        // Task failed to complete
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{task_name} failed to complete"
            );
        }
    }
}
