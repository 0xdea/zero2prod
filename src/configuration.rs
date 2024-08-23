use config::{Config, File, FileFormat};

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub web_port: u16,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub db_port: u16,
    pub host: String,
    pub db_name: String,
}

/// Get settings from configuration file
pub fn get_config() -> Result<Settings, config::ConfigError> {
    Config::builder()
        .add_source(File::new("config.yaml", FileFormat::Yaml))
        .build()?
        .try_deserialize()
}
