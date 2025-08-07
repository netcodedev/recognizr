use crate::error::AppError;
use crate::models::{DebugParams, DetectedFace, FinalResult, Person, RecognitionResult};
use crate::pipeline::{detect_faces, draw_detections, get_recognition_embedding, X_OFFSET, Y_OFFSET};
use crate::AppState;
use axum::routing::post;
use axum::{
    extract::{DefaultBodyLimit, Multipart, Query, State},
    http::{header, HeaderMap, StatusCode},
    Json,
};
use image::{DynamicImage, GenericImageView};
use tracing::debug;
use std::sync::Arc;
use std::time::Instant;

pub fn create_router() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/enroll", post(enroll_handler))
        .route("/recognize", post(recognize_handler))
        .route("/debug/detector", axum::routing::post(debug_detector_handler))
        .layer(DefaultBodyLimit::max(15 * 1024 * 1024)) // 15MB limit for image uploads
}

async fn enroll_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DebugParams>,
    multipart: Multipart,
) -> Result<StatusCode, AppError> {
    let (name, image_bytes) = parse_enroll_multipart(multipart).await?;

    // Validate name is not empty
    if name.trim().is_empty() {
        return Err(AppError::BadRequest("Name cannot be empty".to_string()));
    }

    // Validate image size
    if image_bytes.is_empty() {
        return Err(AppError::BadRequest("Image data is empty".to_string()));
    }

    let original_image = image::load_from_memory(&image_bytes)?;
    let (original_w, original_h) = original_image.dimensions();

    let (mut faces, new_w, new_h) = {
        let mut detector_session_guard = state.detector_session.lock().unwrap();
        detect_faces(&mut detector_session_guard, &image_bytes, &params)?
    };

    if faces.len() != 1 {
        return Err(AppError::BadRequest(format!(
            "Enrollment requires exactly 1 face, but {} were found.",
            faces.len()
        )));
    }

    let scale_w = original_w as f32 / new_w as f32;
    let scale_h = original_h as f32 / new_h as f32;

    let face = &mut faces[0];
    face.scale_to_original(scale_w, scale_h, X_OFFSET, Y_OFFSET);

    let embedding = {
        let mut recognizer_session_guard = state.recognizer_session.lock().unwrap();
        get_recognition_embedding(&mut recognizer_session_guard, &original_image, face)?
    };

    let person = Person { name, embedding };
    let _created_person: Option<Person> = state.db.create("person").content(person).await?;

    Ok(StatusCode::CREATED)
}

async fn recognize_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DebugParams>,
    multipart: Multipart,
) -> Result<Json<Vec<RecognitionResult>>, AppError> {
    let image_bytes = parse_recognize_multipart(multipart).await?;

    // Validate image size
    if image_bytes.is_empty() {
        return Err(AppError::BadRequest("Image data is empty".to_string()));
    }

    let original_image = image::load_from_memory(&image_bytes)?;
    let (original_w, original_h) = original_image.dimensions();

    let (mut faces, new_w, new_h) = {
        let mut detector_session_guard = state.detector_session.lock().unwrap();
        detect_faces(&mut detector_session_guard, &image_bytes, &params)?
    };
    if faces.is_empty() {
        return Ok(Json(Vec::new()));
    }
    let scale_w = original_w as f32 / new_w as f32;
    let scale_h = original_h as f32 / new_h as f32;

    let mut results = Vec::new();
    for face in &mut faces {
        face.scale_to_original(scale_w, scale_h, X_OFFSET, Y_OFFSET);
        let embedding = {
            let mut recognizer_session_guard = state.recognizer_session.lock().unwrap();
            get_recognition_embedding(&mut recognizer_session_guard, &original_image, &face)?
        };

        let mut response = state.db
            .query("SELECT name, vector::similarity::cosine(embedding, $query) AS similarity FROM person ORDER BY similarity DESC LIMIT 1")
            .bind(("query", embedding))
            .await?;

        if let Some(mut db_res) = response.take::<Option<RecognitionResult>>(0)? {
            db_res.bbox = Some(face.bbox);
            results.push(db_res);
        }
    }

    Ok(Json(results))
}

async fn debug_detector_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DebugParams>,
    multipart: Multipart,
) -> Result<(HeaderMap, Vec<u8>), AppError> {
    let request_start_time = Instant::now();

    // --- 1. Image Loading & Parsing ---
    let image_load_start = Instant::now();
    let image_bytes = parse_recognize_multipart(multipart).await?;

    // Validate image size
    if image_bytes.is_empty() {
        return Err(AppError::BadRequest("Image data is empty".to_string()));
    }

    let mut image = image::load_from_memory(&image_bytes)?;
    let (original_w, original_h) = image.dimensions();
    debug!("Image loaded in {} ms", image_load_start.elapsed().as_millis());

    // --- 2. Detect all faces in the image ---
    let detection_start = Instant::now();
    let (detected_faces, new_w, new_h) = {
        let mut detector_session_guard = state.detector_session.lock().unwrap();
        detect_faces(&mut detector_session_guard, &image_bytes, &params)?
    };
    debug!("Face detection completed in {} ms", detection_start.elapsed().as_millis());

    let mut final_results = Vec::new();

    // 2. For each detected face, run recognition
    let faces_recognition_start = Instant::now();
    let scale_w = original_w as f32 / new_w as f32;
    let scale_h = original_h as f32 / new_h as f32;

    for face in detected_faces {
        let result = process_detected_face(&state, face, &image, scale_w, scale_h).await?;
        final_results.push(result);
    }
    debug!("All faces processed in {} ms", faces_recognition_start.elapsed().as_millis());

    // 3. Draw the final results (boxes, dots, AND labels)
    let draw_start = Instant::now();
    draw_detections(&mut image, &final_results, &state.font);
    debug!("Drawing completed in {} ms", draw_start.elapsed().as_millis());

    // 4. Encode and return the image
    let encode_start = Instant::now();
    let mut buffer = std::io::Cursor::new(Vec::new());
    image.write_to(&mut buffer, image::ImageFormat::Png)?;
    let response_bytes = buffer.into_inner();
    debug!("Image encoding completed in {} ms", encode_start.elapsed().as_millis());
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());
    debug!("Total request time: {} ms", request_start_time.elapsed().as_millis());
    debug!("--------------------------");
    Ok((headers, response_bytes))
}

/// Process a single detected face: scale coordinates, generate embedding, and query database
async fn process_detected_face(
    state: &AppState,
    mut face: DetectedFace,
    original_image: &DynamicImage,
    scale_w: f32,
    scale_h: f32,
) -> Result<FinalResult, AppError> {
    let face_recognition_start = Instant::now();

    // Scale coordinates back to original image space
    face.scale_to_original(scale_w, scale_h, X_OFFSET, Y_OFFSET);

    // Validate that the face coordinates are within image bounds
    let (image_width, image_height) = original_image.dimensions();
    if !face.validate_bounds(image_width, image_height) {
        debug!("Face coordinates are out of bounds, skipping recognition");
        return Ok(FinalResult { detection: face, recognition: None });
    }

    // Generate embedding
    let embedding_start = Instant::now();
    let embedding = {
        let mut recognizer_session_guard = state.recognizer_session.lock().unwrap();
        get_recognition_embedding(&mut recognizer_session_guard, original_image, &face)?
    };
    debug!("Face embedding computed in {} ms", embedding_start.elapsed().as_millis());

    // Query database for recognition
    let db_query_start = Instant::now();
    let mut response = state.db
        .query("SELECT name, vector::similarity::cosine(embedding, $query) AS similarity FROM person ORDER BY similarity DESC LIMIT 1")
        .bind(("query", embedding))
        .await?;
    debug!("DB query completed in {} ms", db_query_start.elapsed().as_millis());

    let recognition: Option<(String, f32)> = response.take::<Option<RecognitionResult>>(0)?
        .map(|r| (r.name, r.similarity));

    debug!("Face recognition completed in {} ms", face_recognition_start.elapsed().as_millis());

    Ok(FinalResult { detection: face, recognition })
}

async fn parse_enroll_multipart(
    mut multipart: Multipart,
) -> Result<(String, Vec<u8>), AppError> {
    let mut name = None;
    let mut image_bytes = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "name" {
            name = Some(field.text().await.map_err(|e| {
                AppError::BadRequest(format!("Failed to read name field: {}", e))
            })?);
        } else if field_name == "image" {
            image_bytes = Some(field.bytes().await.map_err(|e| {
                AppError::BadRequest(format!("Failed to read image field: {}", e))
            })?.to_vec());
        }
    }

    let name = name.ok_or_else(|| AppError::MissingMultipartField("name".to_string()))?;
    let image_bytes = image_bytes.ok_or_else(|| AppError::MissingMultipartField("image".to_string()))?;
    Ok((name, image_bytes))
}

async fn parse_recognize_multipart(mut multipart: Multipart) -> Result<Vec<u8>, AppError> {
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        if field.name().unwrap_or("") == "image" {
            return Ok(field.bytes().await.map_err(|e| {
                AppError::BadRequest(format!("Failed to read image field: {}", e))
            })?.to_vec());
        }
    }
    Err(AppError::MissingMultipartField("image".to_string()))
}