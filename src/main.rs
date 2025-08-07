use ort::{execution_providers::CUDAExecutionProvider, session::{builder::SessionBuilder, Session}};
use ab_glyph::FontArc;
use std::{fs, sync::{Arc, Mutex}};
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Surreal,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod handlers;
mod models;
mod pipeline;

use config::{ModelMetadata, DetectorMetadata, extract_detector_metadata, extract_recognizer_metadata, create_detector_metadata_with_mappings};

pub struct AppState {
    db: Surreal<Client>,
    detector_session: Mutex<Session>,
    recognizer_session: Mutex<Session>,
    font: FontArc,
    detector_metadata: DetectorMetadata,
    recognizer_metadata: ModelMetadata,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "face_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // --- Load Configuration ---
    tracing::info!("Loading configuration...");
    let config = config::Configuration::load()?;
    tracing::info!("Configuration loaded successfully.");

    // --- Load the Font ---
    tracing::info!("Loading font from: {:?}", config.font.path);
    let font_data = fs::read(&config.font.path)?;
    let font = FontArc::try_from_vec(font_data)?;
    tracing::info!("Font loaded successfully.");

    // --- Load Models ---
    ort::init()
        .with_execution_providers([CUDAExecutionProvider::default().build()])
        .commit()?;
    
    tracing::info!("Loading models...");
    tracing::info!("Loading detector from: {:?}", config.models.detector.path);
    let mut detector_session = SessionBuilder::new()?
        .commit_from_file(&config.models.detector.path)?;
    tracing::info!("Loading recognizer from: {:?}", config.models.recognizer.path);
    let recognizer_session = SessionBuilder::new()?
        .commit_from_file(&config.models.recognizer.path)?;
    tracing::info!("Models loaded successfully.");

    // --- Extract Model Metadata ---
    tracing::info!("Extracting model metadata...");
    let basic_detector_metadata = extract_detector_metadata(&detector_session, &config.models.detector)?;
    let recognizer_metadata = extract_recognizer_metadata(&recognizer_session, &config.models.recognizer)?;

    // --- Pre-compute Output Mappings ---
    tracing::info!("Pre-computing detector output mappings...");
    let stride_output_mapping = pipeline::match_outputs_by_shape_at_startup(
        &mut detector_session,
        &basic_detector_metadata.output_names,
        &config.models.detector.strides,
        config.models.detector.input_shape[0],
        config.models.detector.input_shape[1],
    )?;

    let detector_metadata = create_detector_metadata_with_mappings(basic_detector_metadata, stride_output_mapping);

    // Check if we have the expected number of outputs for the strides
    let expected_outputs = config.models.detector.strides.len() * 3; // 3 outputs per stride
    if detector_metadata.output_names.len() != expected_outputs {
        tracing::warn!("Expected {} outputs for {} strides, but got {}. This may cause issues.",
                      expected_outputs, config.models.detector.strides.len(), detector_metadata.output_names.len());
    }

    // --- Connect to SurrealDB ---
    let db = Surreal::new::<Ws>(config.database_url()).await?;
    db.signin(Root {
        username: &config.database.username,
        password: &config.database.password,
    })
    .await?;
    db.use_ns(&config.database.namespace).use_db(&config.database.database).await?;
    tracing::info!("Database connection established.");

    // --- Create Application State ---
    let shared_state = Arc::new(AppState {
        db,
        detector_session: Mutex::new(detector_session),
        recognizer_session: Mutex::new(recognizer_session),
        font,
        detector_metadata,
        recognizer_metadata,
    });

    // --- Run Server ---
    let app = handlers::create_router().with_state(shared_state);
    let server_address = config.server_address();
    let listener = tokio::net::TcpListener::bind(&server_address).await?;
    tracing::info!("Server listening on {}", listener.local_addr()?);
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}