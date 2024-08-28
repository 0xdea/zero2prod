use config::{Config, ConfigError, Environment, File};
use secrecy::{ExposeSecret, Secret};

/// Settings
#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
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
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
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
            Env::Production => "prod",
        }
    }
}

impl TryFrom<String> for Env {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "dev" => Ok(Self::Development),
            "prod" => Ok(Self::Production),
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

impl DatabaseSettings {
    /// Generate connection string from database settings
    pub fn database_url(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.db_host,
            self.db_port,
            self.db_name
        ))
    }
}
