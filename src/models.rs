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

impl DetectedFace {
    /// Scale face coordinates back to original image space and apply offsets
    pub fn scale_to_original(&mut self, scale_w: f32, scale_h: f32, x_offset: f32, y_offset: f32) {
        // Scale bounding box coordinates
        self.bbox[0] = (self.bbox[0] * scale_w) - x_offset; // x1
        self.bbox[2] = (self.bbox[2] * scale_w) - x_offset; // x2
        self.bbox[1] = (self.bbox[1] * scale_h) - y_offset; // y1
        self.bbox[3] = (self.bbox[3] * scale_h) - y_offset; // y2

        // Scale keypoints
        self.kps.iter_mut().for_each(|point| {
            point[0] = (point[0] * scale_w) - x_offset; // x
            point[1] = (point[1] * scale_h) - y_offset; // y
        });
    }

    /// Validate that bounding box coordinates are within image bounds
    pub fn validate_bounds(&self, image_width: u32, image_height: u32) -> bool {
        self.bbox[0] >= 0.0
            && self.bbox[1] >= 0.0
            && self.bbox[2] <= image_width as f32
            && self.bbox[3] <= image_height as f32
            && self.bbox[0] < self.bbox[2]
            && self.bbox[1] < self.bbox[3]
    }

    /// Get safe crop coordinates, ensuring they're within bounds
    pub fn get_safe_crop_coords(&self, image_width: u32, image_height: u32) -> (u32, u32, u32, u32) {
        let x1 = self.bbox[0].max(0.0).round() as u32;
        let y1 = self.bbox[1].max(0.0).round() as u32;
        let x2 = self.bbox[2].min(image_width as f32).round() as u32;
        let y2 = self.bbox[3].min(image_height as f32).round() as u32;

        let width = x2.saturating_sub(x1).max(1);
        let height = y2.saturating_sub(y1).max(1);

        (x1, y1, width, height)
    }
}

/// Represents the final result for a recognized face.
#[derive(Debug, Serialize, Deserialize)]
pub struct RecognitionResult {
    pub name: String,
    pub similarity: f32,
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub bbox: Option<[f32; 4]>,
}

pub struct FinalResult {
    pub detection: DetectedFace,
    pub recognition: Option<(String, f32)>, // (Name, Similarity Score)
}

#[derive(Debug, Deserialize)]
pub struct DebugParams {
    // You can call /debug/detector?threshold=0.6
    pub threshold: Option<f32>,
}