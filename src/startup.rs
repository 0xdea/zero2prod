use std::{io, net, time};

use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::middleware::from_fn;
use actix_web::{web, App, HttpServer};
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use secrecy::{ExposeSecret, SecretString};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::authentication::reject_logged_out_users;
use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::{
    confirm, dashboard, healthcheck, home, login, login_form, logout, newsletters,
    newsletters_form, password, password_form, subscriptions,
};

/// Application base URL
pub struct ApplicationBaseUrl(pub String);

/// Application
pub struct Application {
    server: Server,
    port: u16,
}

impl Application {
    /// Build an application based on settings
    pub async fn build(config: Settings) -> anyhow::Result<Self> {
        // Connect to the database
        let db_pool = PgPoolOptions::new()
            .acquire_timeout(time::Duration::from_secs(2))
            .connect_lazy_with(config.database.db_options());

        // Run the HTTP server and return its data
        Self::build_with_db_pool(config, &db_pool).await
    }

    /// Build an application based on settings and database pool
    pub async fn build_with_db_pool(config: Settings, db_pool: &PgPool) -> anyhow::Result<Self> {
        // Build the email client
        let email_client = config.email_client.client();

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
            config.application.signing_key,
            config.redis_uri,
        )
        .await?;
        Ok(Self { server, port })
    }

    /// Get application port
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Run application until it is stopped
    pub async fn run_until_stopped(self) -> io::Result<()> {
        self.server.await
    }
}

/// Run the HTTP server
pub async fn run_server(
    listener: net::TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    signing_key: SecretString,
    redis_uri: SecretString,
) -> anyhow::Result<Server> {
    // Extract secret key from HMAC secret
    let signing_key = Key::from(signing_key.expose_secret().as_bytes());

    // Build message framework
    let message_store = CookieMessageStore::builder(signing_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();

    // Set up Redis session store
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;

    // Prepare data to be added the application context
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));

    // Start the HTTP server
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                signing_key.clone(),
            ))
            .wrap(TracingLogger::default())
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/healthcheck", web::get().to(healthcheck))
            .route("/subscriptions", web::post().to(subscriptions))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .service(
                web::scope("/admin")
                    .wrap(from_fn(reject_logged_out_users))
                    .route("/dashboard", web::get().to(dashboard))
                    .route("/newsletters", web::get().to(newsletters_form))
                    .route("/newsletters", web::post().to(newsletters))
                    .route("/password", web::get().to(password_form))
                    .route("/password", web::post().to(password))
                    .route("/logout", web::post().to(logout)),
            )
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run())
}
