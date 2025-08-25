use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ort::session::Session;
use crate::error::AppError;

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
    pub detector: DetectorConfig,
    pub recognizer: RecognizerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    pub path: PathBuf,
    pub strides: Vec<i32>,
    /// Input shape for the detector model [height, width]
    pub input_shape: [u32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognizerConfig {
    pub path: PathBuf,
    /// Input size for the recognizer model (square input)
    pub input_size: u32,
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
                detector: DetectorConfig {
                    path: PathBuf::from("models/scrfd_10g_bnkps.onnx"),
                    strides: vec![8, 16, 32],
                    input_shape: [640, 640],
                },
                recognizer: RecognizerConfig {
                    path: PathBuf::from("models/arcface_r100.onnx"),
                    input_size: 112,
                },
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

/// Metadata extracted from a model
#[derive(Debug, Clone)]
pub struct ModelMetadata {
    pub input_name: String,
    pub input_shape: Vec<i64>,
    pub output_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DetectorMetadata {
    pub input_name: String,
    pub input_shape: Vec<i64>,
    pub output_names: Vec<String>,
    /// Pre-computed output mapping: stride -> (score_idx, bbox_idx, kps_idx)
    pub stride_output_mapping: std::collections::HashMap<i32, (usize, usize, usize)>,
}

/// Extract basic metadata from a detector model session
pub fn extract_detector_metadata(session: &Session, config: &DetectorConfig) -> Result<ModelMetadata, AppError> {
    // Extract input information
    let input = session.inputs.first()
        .ok_or_else(|| AppError::BadRequest("Model has no inputs".to_string()))?;

    let input_name = input.name.clone();

    // Use configured input shape (model metadata extraction can be unreliable)
    let input_shape = vec![1, 3, config.input_shape[0] as i64, config.input_shape[1] as i64];
    tracing::debug!("Using configured input shape: {:?}", input_shape);

    // Extract output information
    let mut output_names = Vec::new();

    for output in &session.outputs {
        output_names.push(output.name.clone());
    }

    Ok(ModelMetadata {
        input_name,
        input_shape,
        output_names,
    })
}

/// Create detector metadata with pre-computed output mappings
pub fn create_detector_metadata_with_mappings(
    basic_metadata: ModelMetadata,
    stride_output_mapping: std::collections::HashMap<i32, (usize, usize, usize)>,
) -> DetectorMetadata {
    tracing::debug!("Detector model metadata:");
    tracing::debug!("  Input: {} {:?}", basic_metadata.input_name, basic_metadata.input_shape);
    tracing::debug!("  Outputs: {} total", basic_metadata.output_names.len());
    for (i, name) in basic_metadata.output_names.iter().enumerate() {
        tracing::debug!("    Output {}: {}", i, name);
    }
    tracing::debug!("  Pre-computed mappings for {} strides", stride_output_mapping.len());

    DetectorMetadata {
        input_name: basic_metadata.input_name,
        input_shape: basic_metadata.input_shape,
        output_names: basic_metadata.output_names,
        stride_output_mapping,
    }
}

/// Extract metadata from a recognizer model session with configured input size
pub fn extract_recognizer_metadata(session: &Session, config: &RecognizerConfig) -> Result<ModelMetadata, AppError> {
    // Extract input information
    let input = session.inputs.first()
        .ok_or_else(|| AppError::BadRequest("Model has no inputs".to_string()))?;

    let input_name = input.name.clone();

    // Use configured input size (model metadata extraction can be unreliable)
    let input_shape = vec![1, 3, config.input_size as i64, config.input_size as i64];
    tracing::debug!("Using configured recognizer input shape: {:?}", input_shape);

    // Extract output information
    let mut output_names = Vec::new();

    for output in &session.outputs {
        output_names.push(output.name.clone());
    }

    Ok(ModelMetadata {
        input_name,
        input_shape,
        output_names,
    })
}
