use crate::domain::Email;
use config::{Config, ConfigError, Environment, File};
use secrecy::{ExposeSecret, Secret};
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::ConnectOptions;
use tracing::log::LevelFilter;
use url::Url;

/// Settings
#[derive(serde::Deserialize)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub email_client: EmailClientSettings,
}

/// Application settings
#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    pub app_host: String,
    pub app_port: u16,
}

/// Database settings
#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    /// Generate connection string from database settings (does not support SSL mode)
    #[deprecated(since = "0.1.1", note = "use `db_options` instead")]
    pub fn db_url(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database
        ))
    }

    /// Generate options and flags that can be used to configure a database connection
    pub fn db_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .username(&self.username)
            .password(self.password.expose_secret())
            .host(&self.host)
            .port(self.port)
            .database(&self.database)
            .ssl_mode(ssl_mode)
            .log_statements(LevelFilter::Trace)
    }
}

/// Email client settings
#[derive(serde::Deserialize)]
pub struct EmailClientSettings {
    #[serde(with = "url_serde")]
    pub base_url: Url,
    pub sender_email: String,
}

impl EmailClientSettings {
    /// Parse sender email
    pub fn sender(&self) -> Result<Email, String> {
        Email::parse(self.sender_email.clone())
    }
}

/// Possible runtime environments
pub enum Env {
    Development,
    Production,
}

impl Env {
    /// Represent Env as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Env::Development => "dev",
            Env::Production => "prd",
        }
    }
}

impl TryFrom<String> for Env {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "dev" => Ok(Self::Development),
            "prd" => Ok(Self::Production),
            other => Err(format!(
                "`{other}` is not a supported environment. Use either `dev` or `prod`"
            )),
        }
    }
}

/// Get settings from configuration files
pub fn get_config() -> Result<Settings, ConfigError> {
    let path = std::env::current_dir().expect("Failed to determine the current directory");
    let config_dir = path.join("config");

    // Detect the running environment (default: `dev`)
    let env: Env = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "dev".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");

    // Read the configuration from files and environment variables
    Config::builder()
        // Base configuration file
        .add_source(File::from(config_dir.join("base.yaml")).required(true))
        // Environment-specific configuration file
        .add_source(File::from(config_dir.join(env.as_str())).required(true))
        // Environment variables (e.g., `ZERO2PROD__APPLICATION__APP_PORT=8888`
        // would set Settings.application.app_port to 8888)
        .add_source(Environment::with_prefix("zero2prod").separator("__"))
        .build()?
        .try_deserialize()
}
