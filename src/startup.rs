use std::{io, net, time};

use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::middleware::from_fn;
use actix_web::{web, App, HttpServer};
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use secrecy::{ExposeSecret, SecretBox};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::authentication::reject_logged_out_users;
use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::{
    confirm, dashboard, healthcheck, home, login, login_form, logout, newsletters, password,
    password_form, subscriptions,
};

/// Application data
pub struct Application {
    server: Server,
    port: u16,
}

/// Application base URL
pub struct ApplicationBaseUrl(pub String);

/// HMAC secret type
// TODO: change name?
pub struct HmacSecret(pub SecretBox<String>);

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
/// TODO: Refactor `HmacSecret` into a more generic secret key new-type
pub async fn run_server(
    listener: net::TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: SecretBox<String>,
    redis_uri: SecretBox<String>,
) -> anyhow::Result<Server> {
    // Extract secret key from HMAC secret
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());

    // Build message framework
    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
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
                secret_key.clone(),
            ))
            .wrap(TracingLogger::default())
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/healthcheck", web::get().to(healthcheck))
            .route("/newsletters", web::post().to(newsletters))
            .route("/subscriptions", web::post().to(subscriptions))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .service(
                web::scope("/admin")
                    .wrap(from_fn(reject_logged_out_users))
                    .route("/dashboard", web::get().to(dashboard))
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
