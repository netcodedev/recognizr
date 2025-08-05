use crate::error::AppError;
use crate::models::{DebugParams, Person, RecognitionResult};
use crate::pipeline::{detect_faces, draw_detections, get_recognition_embedding};
use crate::AppState;
use axum::routing::post;
use axum::{
    extract::{DefaultBodyLimit, Multipart, Query, State},
    http::{header, HeaderMap, StatusCode},
    Json,
};
use image::GenericImageView;
use std::sync::Arc;

const X_OFFSET: f32 = 50.0;
const Y_OFFSET: f32 = 50.0;

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
    face.bbox[0] = (face.bbox[0] * scale_w) - X_OFFSET; // x1
    face.bbox[2] = (face.bbox[2] * scale_w) - X_OFFSET; // x2
    face.bbox[1] = (face.bbox[1] * scale_h) - Y_OFFSET; // y1
    face.bbox[3] = (face.bbox[3] * scale_h) - Y_OFFSET; // y2

    face.kps.iter_mut().for_each(|point| {
        point[0] = (point[0] * scale_w) - X_OFFSET; // x
        point[1] = (point[1] * scale_h) - Y_OFFSET; // y
    });

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
        face.bbox[0] = (face.bbox[0] * scale_w) - X_OFFSET; // x1
        face.bbox[2] = (face.bbox[2] * scale_w) - X_OFFSET; // x2
        face.bbox[1] = (face.bbox[1] * scale_h) - Y_OFFSET; // y1
        face.bbox[3] = (face.bbox[3] * scale_h) - Y_OFFSET; // y2

        face.kps.iter_mut().for_each(|point| {
            point[0] = (point[0] * scale_w) - X_OFFSET; // x
            point[1] = (point[1] * scale_h) - Y_OFFSET; // y
        });
        let embedding = {
            let mut recognizer_session_guard = state.recognizer_session.lock().unwrap();
            get_recognition_embedding(&mut recognizer_session_guard, &original_image, &face)?
        };

        let mut response = state.db
            .query("SELECT name, vector::distance::euclidean(embedding, $query) AS distance FROM person ORDER BY distance ASC LIMIT 1")
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
    let image_bytes = parse_recognize_multipart(multipart).await?;
    let mut image = image::load_from_memory(&image_bytes)?;
    let (original_w, original_h) = image.dimensions();

    let (mut faces, new_w, new_h) = {
        let mut detector_session_guard = state.detector_session.lock().unwrap();
        detect_faces(&mut detector_session_guard, &image_bytes, &params)?
    };

    let scale_w = original_w as f32 / new_w as f32;
    let scale_h = original_h as f32 / new_h as f32;

    for face in &mut faces {
        face.bbox[0] = (face.bbox[0] * scale_w) - X_OFFSET; // x1
        face.bbox[2] = (face.bbox[2] * scale_w) - X_OFFSET; // x2
        face.bbox[1] = (face.bbox[1] * scale_h) - Y_OFFSET; // y1
        face.bbox[3] = (face.bbox[3] * scale_h) - Y_OFFSET; // y2

        face.kps.iter_mut().for_each(|point| {
            point[0] = (point[0] * scale_w) - X_OFFSET; // x
            point[1] = (point[1] * scale_h) - Y_OFFSET; // y
        });
    }

    draw_detections(&mut image, &faces);

    let mut buffer = std::io::Cursor::new(Vec::new());
    image.write_to(&mut buffer, image::ImageFormat::Png)?;
    let response_bytes = buffer.into_inner();

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());

    Ok((headers, response_bytes))
}

async fn parse_enroll_multipart(
    mut multipart: Multipart,
) -> Result<(String, Vec<u8>), AppError> {
    let mut name = None;
    let mut image_bytes = None;
    while let Some(field) = multipart.next_field().await.unwrap() {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "name" {
            name = Some(field.text().await.unwrap());
        } else if field_name == "image" {
            image_bytes = Some(field.bytes().await.unwrap().to_vec());
        }
    }
    let name = name.ok_or_else(|| AppError::MissingMultipartField("name".to_string()))?;
    let image_bytes =
        image_bytes.ok_or_else(|| AppError::MissingMultipartField("image".to_string()))?;
    Ok((name, image_bytes))
}

async fn parse_recognize_multipart(mut multipart: Multipart) -> Result<Vec<u8>, AppError> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        if field.name().unwrap_or("") == "image" {
            return Ok(field.bytes().await.unwrap().to_vec());
        }
    }
    Err(AppError::MissingMultipartField("image".to_string()))
}