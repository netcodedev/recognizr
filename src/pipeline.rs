use crate::{error::AppError, models::DebugParams};
use crate::models::DetectedFace;
use image::{DynamicImage, GenericImageView};
use ndarray::Array;
use ort::{inputs, session::{Session, SessionOutputs}, value::Value};
use imageproc::drawing::draw_hollow_rect_mut;
use imageproc::rect::Rect;
use tracing::debug;

const NMS_THRESHOLD: f32 = 0.4;
const RECOGNIZER_INPUT_SIZE: u32 = 112;

/// Takes raw image bytes, runs the detector, and returns a clean list of faces.
pub fn detect_faces(
    session: &mut Session,
    image_bytes: &[u8],
    params: &DebugParams,
) -> Result<Vec<DetectedFace>, AppError> {
    let image = image::load_from_memory(image_bytes)?;
    let (img_width, img_height) = image.dimensions();

    let use_bgr = params.order.as_deref() == Some("bgr");

    let mut input_tensor =
        Array::zeros((1, 3, img_height as usize, img_width as usize));

    for (x, y, pixel) in image.to_rgb8().enumerate_pixels() {
        let (r, g, b) = (pixel[0], pixel[1], pixel[2]);
        let (ch0, ch1, ch2) = if use_bgr { (b, g, r) } else { (r, g, b) };

        input_tensor[[0, 0, y as usize, x as usize]] = (ch0 as f32 - 127.5) / 127.5;
        input_tensor[[0, 1, y as usize, x as usize]] = (ch1 as f32 - 127.5) / 127.5;
        input_tensor[[0, 2, y as usize, x as usize]] = (ch2 as f32 - 127.5) / 127.5;
    }

    let inputs = inputs![Value::from_array(input_tensor)?];
    let outputs = session.run(inputs)?;

    let proposals = decode_and_filter_proposals(&outputs, img_width as f32, img_height as f32, params)?;
    let final_faces = non_maximum_suppression(&proposals, NMS_THRESHOLD);

    Ok(final_faces)
}

pub fn draw_detections(
    image: &mut DynamicImage,
    detections: &[DetectedFace],
) {
    debug!("Drawing {} detections on image", detections.len());

    const THICKNESS: u32 = 3;

    for face in detections {
        // Generate a random color for each box
        let color = image::Rgba([255, 0, 0, 255]);

        // Create a rectangle from the bounding box coordinates
        let x = face.bbox[0] as i32;
        let y = face.bbox[1] as i32;
        let width = (face.bbox[2] - face.bbox[0]) as u32;
        let height = (face.bbox[3] - face.bbox[1]) as u32;
        
        // Loop to draw multiple rectangles for thickness
        for i in 0..THICKNESS {
            // Create a rectangle inset by `i` pixels
            let rect = Rect::at(x + i as i32, y + i as i32)
                .of_size(width - (i * 2), height - (i * 2));

            // Draw the hollow rectangle on the image
            draw_hollow_rect_mut(image, rect, color);
        }
    }
}

/// Decodes raw model output into candidate faces.
fn decode_and_filter_proposals(
    outputs: &SessionOutputs,
    img_width: f32,
    img_height: f32,
    params: &DebugParams,
) -> Result<Vec<DetectedFace>, AppError> {
    let mut proposals = Vec::new();

    // SCRFD has 3 strides: 8, 16, 32
    let strides = [8, 16, 32];
    let decode_mode = params.decode.as_deref().unwrap_or("ltrb");
    let conf_threshold = params.threshold.unwrap_or(0.5);
    for (i, &stride) in strides.iter().enumerate() {
        let (_scores_shape, scores_data) = outputs[i].try_extract_tensor::<f32>()?;
        let (_boxes_shape, boxes_data) = outputs[i + 3].try_extract_tensor::<f32>()?;
        let (_kps_shape, kps_data) = outputs[i + 6].try_extract_tensor::<f32>()?;

        let feature_height = (img_height / stride as f32).ceil() as usize;
        let feature_width = (img_width / stride as f32).ceil() as usize;

        // There are 2 anchors per location for SCRFD
        for anchor_idx in 0..2 {
            for y in 0..feature_height {
                for x in 0..feature_width {
                    let anchor_center_x = (x as f32 + 0.5) * stride as f32;
                    let anchor_center_y = (y as f32 + 0.5) * stride as f32;

                    let idx = y * feature_width * 2 + x * 2 + anchor_idx;

                    // scores_data shape: [N, 1], N = feature_height * feature_width * 2
                    let score = scores_data[idx];

                    if score < conf_threshold {
                        continue;
                    }

                    // boxes_data shape: [N, 4]
                    let box_base = idx * 4;
                    let box_preds = &boxes_data[box_base..box_base + 4];

                    // kps_data shape: [N, 10]
                    let kps_base = idx * 10;
                    let kps_preds = &kps_data[kps_base..kps_base + 10];

                    // Decode bounding box
                    let (x1_raw, y1_raw, x2_raw, y2_raw) = if decode_mode == "center_wh" {
                        // Alternative decoding: center offset, width, and height
                        let dx = box_preds[0] * stride as f32;
                        let dy = box_preds[1] * stride as f32;
                        let dw = box_preds[2].exp() * stride as f32;
                        let dh = box_preds[3].exp() * stride as f32;
                        let cx = anchor_center_x + dx;
                        let cy = anchor_center_y + dy;
                        (cx - dw * 0.5, cy - dh * 0.5, cx + dw * 0.5, cy + dh * 0.5)
                    } else {
                        // Default decoding: left, top, right, bottom distances
                        let l = box_preds[0].exp() * stride as f32;
                        let t = box_preds[1].exp() * stride as f32;
                        let r = box_preds[2].exp() * stride as f32;
                        let b = box_preds[3].exp() * stride as f32;
                        (anchor_center_x - l, anchor_center_y - t, anchor_center_x + r, anchor_center_y + b)
                    };
                    let x1 = x1_raw.max(0.0);
                    let y1 = y1_raw.max(0.0);
                    let x2 = x2_raw.min(img_width - 1.0);
                    let y2 = y2_raw.min(img_height - 1.0);
                    if x1 >= x2 || y1 >= y2 { continue; }
                    let bbox = [x1, y1, x2, y2];
                    debug!("Decoded bbox: {:?}", bbox);

                    // Decode keypoints
                    let mut kps = [[0.0; 2]; 5];
                    for j in 0..5 {
                        kps[j][0] = anchor_center_x + kps_preds[j * 2] * stride as f32;
                        kps[j][1] = anchor_center_y + kps_preds[j * 2 + 1] * stride as f32;
                    }

                    proposals.push(DetectedFace { bbox, kps, score });
                }
            }
        }
    }
    Ok(proposals)
}

/// Applies Non-Maximum Suppression to filter overlapping boxes.
fn non_maximum_suppression(proposals: &[DetectedFace], iou_threshold: f32) -> Vec<DetectedFace> {
    let mut sorted_proposals = proposals.to_vec();
    sorted_proposals.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    let mut final_faces = Vec::new();
    while !sorted_proposals.is_empty() {
        let best = sorted_proposals.remove(0);
        final_faces.push(best.clone());
        
        sorted_proposals.retain(|p| {
            let iou = calculate_iou(&best.bbox, &p.bbox);
            iou < iou_threshold
        });
    }
    final_faces
}

fn calculate_iou(box_a: &[f32; 4], box_b: &[f32; 4]) -> f32 {
    let ix1 = box_a[0].max(box_b[0]);
    let iy1 = box_a[1].max(box_b[1]);
    let ix2 = box_a[2].min(box_b[2]);
    let iy2 = box_a[3].min(box_b[3]);

    let i_width = (ix2 - ix1).max(0.0);
    let i_height = (iy2 - iy1).max(0.0);
    let intersection_area = i_width * i_height;

    let area_a = (box_a[2] - box_a[0]) * (box_a[3] - box_a[1]);
    let area_b = (box_b[2] - box_b[0]) * (box_b[3] - box_b[1]);
    let union_area = area_a + area_b - intersection_area;
    
    if union_area == 0.0 { 0.0 } else { intersection_area / union_area }
}


/// Takes a detected face, crops, aligns, and generates a 512-dim embedding.
pub fn get_recognition_embedding(
    session: &mut Session,
    original_image: &DynamicImage,
    face: &DetectedFace,
) -> Result<Vec<f32>, AppError> {
    // NOTE: A simple crop is used here. For higher accuracy, an affine transformation
    // using the keypoints to align the face should be implemented.
    let cropped_face = original_image.crop_imm(
        face.bbox[0] as u32,
        face.bbox[1] as u32,
        (face.bbox[2] - face.bbox[0]) as u32,
        (face.bbox[3] - face.bbox[1]) as u32,
    );

    let resized = cropped_face.resize_exact(
        RECOGNIZER_INPUT_SIZE,
        RECOGNIZER_INPUT_SIZE,
        image::imageops::FilterType::Triangle,
    );

    let mut input_tensor =
        Array::zeros((1, 3, RECOGNIZER_INPUT_SIZE as usize, RECOGNIZER_INPUT_SIZE as usize));
    for (x, y, pixel) in resized.to_rgb8().enumerate_pixels() {
        input_tensor[[0, 0, y as usize, x as usize]] = (pixel[2] as f32 - 127.5) / 127.5;
        input_tensor[[0, 1, y as usize, x as usize]] = (pixel[1] as f32 - 127.5) / 127.5;
        input_tensor[[0, 2, y as usize, x as usize]] = (pixel[0] as f32 - 127.5) / 127.5;
    }

    let inputs = inputs![Value::from_array(input_tensor)?];
    let outputs = session.run(inputs)?;

    let (_, embedding_slice) = outputs[0].try_extract_tensor()?;
    let mut embedding: Vec<f32> = embedding_slice.to_vec();
    let norm = (embedding.iter().map(|v| v.powi(2)).sum::<f32>()).sqrt();
    embedding.iter_mut().for_each(|v| *v /= norm);

    Ok(embedding)
}