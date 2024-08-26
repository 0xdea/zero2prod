use config::{Config, File, FileFormat};
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

/// Get settings from configuration file
pub fn get_config() -> Result<Settings, config::ConfigError> {
    Config::builder()
        .add_source(File::new("config.yaml", FileFormat::Yaml))
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
