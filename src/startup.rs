use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

/// Application data
pub struct Application {
    server: Server,
    port: u16,
}

impl Application {
    /// Build an application based on settings and database pool
    pub async fn build(config: Settings, db_pool: PgPool) -> Result<Self, std::io::Error> {
        // Build an email client
        let base_url = config.email_client.base_url().expect("Invalid base URL");
        let sender_email = config
            .email_client
            .sender_email()
            .expect("Invalid sender email address");
        let email_client = EmailClient::new(
            base_url,
            sender_email,
            config.email_client.authorization_token.clone(),
            config.email_client.timeout(),
        );

        // Run the HTTP server and return its data
        let listener = std::net::TcpListener::bind(format!(
            "{}:{}",
            config.application.app_host, config.application.app_port
        ))?;
        let port = listener.local_addr().unwrap().port();
        let server = run_server(listener, db_pool, email_client)?;
        Ok(Self { server, port })
    }

    /// Get application port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Run application until it is stopped
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

/// Run the HTTP server
pub fn run_server(
    listener: std::net::TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    // Prepare data to be added the application context
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);

    // Start the HTTP server
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run())
}
