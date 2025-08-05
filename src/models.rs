use serde::{Deserialize, Serialize};

/// Represents a person's record in the database.
#[derive(Debug, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub embedding: Vec<f32>,
}

/// Represents a clean, decoded face detection.
#[derive(Debug, Clone)]
pub struct DetectedFace {
    pub bbox: [f32; 4],      // [x1, y1, x2, y2]
    pub kps: [[f32; 2]; 5], // 5 keypoints, each [x, y]
    pub score: f32,
}

/// Represents the final result for a recognized face.
#[derive(Debug, Serialize, Deserialize)]
pub struct RecognitionResult {
    pub name: String,
    pub distance: f32,
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub bbox: Option<[f32; 4]>,
}

#[derive(Debug, Deserialize)]
pub struct DebugParams {
    // You can call /debug/detector?threshold=0.6
    pub threshold: Option<f32>,
    // You can call /debug/detector?order=bgr
    pub order: Option<String>,
    // You can call /debug/detector?decode=center_wh
    pub decode: Option<String>,
}