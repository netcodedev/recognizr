use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub font: FontConfig,
    pub models: ModelsConfig,
    pub database: DatabaseConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    pub detector_path: PathBuf,
    pub recognizer_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub namespace: String,
    pub database: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Configuration {
    pub fn load() -> anyhow::Result<Self> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name("config"))
            .add_source(config::Environment::with_prefix("RECOGNIZR"))
            .build()?;

        let config = settings.try_deserialize()?;
        Ok(config)
    }

    pub fn database_url(&self) -> String {
        format!("{}:{}", self.database.host, self.database.port)
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            font: FontConfig {
                path: PathBuf::from("DejaVuSansMono.ttf"),
            },
            models: ModelsConfig {
                detector_path: PathBuf::from("models/scrfd_10g_bnkps.onnx"),
                recognizer_path: PathBuf::from("models/arcface_r100.onnx"),
            },
            database: DatabaseConfig {
                host: "127.0.0.1".to_string(),
                port: 8000,
                username: "root".to_string(),
                password: "root".to_string(),
                namespace: "test".to_string(),
                database: "test".to_string(),
            },
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
            },
        }
    }
}
