use std::{env, time};

use config::{Config, ConfigError, Environment, File};
use reqwest::Url;
use secrecy::{ExposeSecret, SecretBox};
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::ConnectOptions;
use tracing::log::LevelFilter;
use url::ParseError;

use crate::domain::EmailAddress;

/// Settings
#[derive(serde::Deserialize)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub email_client: EmailClientSettings,
    pub redis_uri: SecretBox<String>,
}

/// Application settings
#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    pub app_host: String,
    pub app_port: u16,
    pub base_url: String,
    pub hmac_secret: SecretBox<String>,
}

/// Database settings
#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    username: String,
    password: SecretBox<String>,
    host: String,
    port: u16,
    database: String,
    require_ssl: bool,
}

impl DatabaseSettings {
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
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: SecretBox<String>,
    pub timeout_millis: u64,
}

impl EmailClientSettings {
    /// Parse base URL
    pub fn base_url(&self) -> Result<Url, ParseError> {
        Url::parse(&self.base_url)
    }

    /// Parse sender email
    pub fn sender_email(&self) -> Result<EmailAddress, String> {
        EmailAddress::parse(self.sender_email.clone())
    }

    /// Get configured timeout
    pub const fn timeout(&self) -> time::Duration {
        time::Duration::from_millis(self.timeout_millis)
    }
}

/// Available runtime environments
pub enum Env {
    Development,
    Production,
}

impl Env {
    /// Represent environment as a string
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Development => "dev",
            Self::Production => "prd",
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
                "`{other}` is not a supported environment. Use either `dev` or `prd`"
            )),
        }
    }
}

/// Get settings from configuration files
pub fn get_config() -> Result<Settings, ConfigError> {
    let path = env::current_dir().expect("Failed to determine the current directory");
    let config_dir = path.join("config");

    // Detect the running environment (default: `dev`)
    let env: Env = env::var("APP_ENVIRONMENT")
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
        .add_source(Environment::with_prefix("ZERO2PROD").separator("__"))
        .build()?
        .try_deserialize()
}
