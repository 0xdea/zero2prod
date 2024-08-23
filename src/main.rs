use std::net::TcpListener;
use zero2prod::configuration::get_config;
use zero2prod::startup::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = get_config().expect("Failed to read configuration");
    run(TcpListener::bind(format!("127.0.0.1:{}", config.web_port))?)?.await
}
