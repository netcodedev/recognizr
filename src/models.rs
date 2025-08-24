use serde::{Deserialize, Serialize};

/// Represents a person's record in the database.
#[derive(Debug, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub embedding: Vec<f32>,
    pub cropped_image: Vec<u8>, // JPEG encoded cropped face image
}

/// Represents a person for gallery display (without embedding data)
#[derive(Debug, Serialize, Deserialize)]
pub struct GalleryPerson {
    pub name: String,
    pub image_base64: String, // Base64 encoded JPEG image
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

    /// Get square crop coordinates with padding around the face for gallery display
    /// Returns coordinates for a square crop that's larger than the bounding box
    pub fn get_square_crop_coords(&self, image_width: u32, image_height: u32, padding_factor: f32) -> (u32, u32, u32) {
        let face_width = (self.bbox[2] - self.bbox[0]).abs();
        let face_height = (self.bbox[3] - self.bbox[1]).abs();

        // Use the larger dimension and add padding
        let base_size = face_width.max(face_height);
        let crop_size = (base_size * (1.0 + padding_factor)).round() as u32;

        // Calculate center of the face
        let center_x = (self.bbox[0] + self.bbox[2]) / 2.0;
        let center_y = (self.bbox[1] + self.bbox[3]) / 2.0;

        // Calculate crop coordinates centered on the face
        let half_size = crop_size / 2;
        let crop_x = (center_x as u32).saturating_sub(half_size).min(image_width.saturating_sub(crop_size));
        let crop_y = (center_y as u32).saturating_sub(half_size).min(image_height.saturating_sub(crop_size));

        // Ensure crop size doesn't exceed image bounds
        let final_crop_size = crop_size.min(image_width - crop_x).min(image_height - crop_y);

        (crop_x, crop_y, final_crop_size)
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