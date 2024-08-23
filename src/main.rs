use sqlx::{Connection, PgConnection};
use std::net::TcpListener;
use zero2prod::configuration::get_config;
use zero2prod::startup::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = get_config().expect("Failed to read configuration");
    let conn = PgConnection::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to the database");
    run(
        TcpListener::bind(format!("127.0.0.1:{}", config.app_port))?,
        conn,
    )?
    .await
}
