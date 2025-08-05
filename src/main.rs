use ort::{session::Session, session::builder::SessionBuilder};
use std::sync::{Arc, Mutex};
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Surreal,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod error;
mod handlers;
mod models;
mod pipeline;

pub struct AppState {
    db: Surreal<Client>,
    detector_session: Mutex<Session>,
    recognizer_session: Mutex<Session>,
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

    // --- Load Models ---
    tracing::info!("Loading models...");
    let detector_session = SessionBuilder::new()?
        .commit_from_file("models/scrfd_2.5g_bnkps.onnx")?;
    let recognizer_session = SessionBuilder::new()?
        .commit_from_file("models/face_recognition.onnx")?;
    tracing::info!("Models loaded successfully.");

    // --- Connect to SurrealDB ---
    tracing::info!("Connecting to database...");
    let db = Surreal::new::<Ws>("127.0.0.1:8000").await?;
    db.signin(Root {
        username: "root",
        password: "root",
    })
    .await?;
    db.use_ns("test").use_db("test").await?;
    tracing::info!("Database connection established.");

    // --- Create Application State ---
    let shared_state = Arc::new(AppState {
        db,
        detector_session: Mutex::new(detector_session),
        recognizer_session: Mutex::new(recognizer_session),
    });

    // --- Run Server ---
    let app = handlers::create_router().with_state(shared_state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Server listening on {}", listener.local_addr()?);
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}