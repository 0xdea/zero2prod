use config::{Config, File, FileFormat};

/// Settings
#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub app_port: u16,
}

/// Database settings
#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
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
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.db_host, self.db_port, self.db_name
        )
    }
}
