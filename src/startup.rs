use std::{io, net, time};

use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use secrecy::{ExposeSecret, SecretBox};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::{
    form, healthcheck, home, login, newsletters, subscriptions, subscriptions_confirm,
};

/// Application data
pub struct Application {
    server: Server,
    port: u16,
}

/// Application base URL
pub struct ApplicationBaseUrl(pub String);

/// HMAC secret type
pub struct HmacSecret(pub SecretBox<String>);

impl Application {
    /// Build an application based on settings
    pub async fn build(config: Settings) -> Result<Self, io::Error> {
        // Connect to the database
        let db_pool = PgPoolOptions::new()
            .acquire_timeout(time::Duration::from_secs(2))
            .connect_lazy_with(config.database.db_options());

        // Run the HTTP server and return its data
        Self::build_with_db_pool(config, &db_pool).await
    }

    /// Build an application based on settings and database pool
    #[allow(clippy::unused_async)]
    pub async fn build_with_db_pool(config: Settings, db_pool: &PgPool) -> Result<Self, io::Error> {
        // Build an email client
        let base_url = config.email_client.base_url().expect("Invalid base URL");
        let sender_email = config
            .email_client
            .sender_email()
            .expect("Invalid sender email address");
        let email_client = EmailClient::new(
            config.email_client.timeout(),
            base_url,
            sender_email,
            config.email_client.authorization_token,
        );

        // Run the HTTP server and return its data
        let listener = net::TcpListener::bind(format!(
            "{}:{}",
            config.application.app_host, config.application.app_port
        ))?;
        let port = listener.local_addr()?.port();
        let server = run_server(
            listener,
            db_pool.clone(),
            email_client,
            config.application.base_url,
            config.application.hmac_secret,
        )?;
        Ok(Self { server, port })
    }

    /// Get application port
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Run application until it is stopped
    pub async fn run_until_stopped(self) -> Result<(), io::Error> {
        self.server.await
    }
}

/// Run the HTTP server
pub fn run_server(
    listener: net::TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: SecretBox<String>,
) -> Result<Server, io::Error> {
    // Build message framework
    let message_store =
        CookieMessageStore::builder(Key::from(hmac_secret.expose_secret().as_bytes())).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();

    // Prepare data to be added the application context
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let hmac_secret = web::Data::new(HmacSecret(hmac_secret));

    // Start the HTTP server
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(message_framework.clone())
            .wrap(TracingLogger::default())
            .route("/", web::get().to(home))
            .route("/login", web::get().to(form))
            .route("/login", web::post().to(login))
            .route("/healthcheck", web::get().to(healthcheck))
            .route("/subscriptions", web::post().to(subscriptions))
            .route(
                "/subscriptions/confirm",
                web::get().to(subscriptions_confirm),
            )
            .route("/newsletters", web::post().to(newsletters))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(hmac_secret.clone())
    })
    .listen(listener)?
    .run())
}
