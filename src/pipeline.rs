use crate::error::AppError;
use crate::models::{DebugParams, DetectedFace, FinalResult};
use image::{imageops, DynamicImage, RgbImage, Rgba};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use ndarray::{s, Array, ArrayBase, Dim, IxDynImpl, ViewRepr};
use ort::{inputs, session::{Session}, value::Value};
use ab_glyph::{FontArc, PxScale};
use tracing::debug;

// --- TUNING PARAMETERS (match these with your Python script) ---
const NMS_THRESHOLD: f32 = 0.4;
const RECOGNIZER_INPUT_SIZE: u32 = 112;
const DETECTOR_TARGET_SHAPE: (u32, u32) = (640, 640); // (height, width)

/// Preprocesses an image using the "top-left" letterbox method.
/// A direct Rust translation of the Python `preprocess_image_topleft` function.
fn preprocess_image_topleft(
    img: &DynamicImage,
    target_height: u32,
    target_width: u32,
) -> (RgbImage, u32, u32) {
    let img_h = img.height();
    let img_w = img.width();

    let ratio = f32::min(
        target_width as f32 / img_w as f32,
        target_height as f32 / img_h as f32,
    );

    let new_w = (img_w as f32 * ratio).round() as u32;
    let new_h = (img_h as f32 * ratio).round() as u32;

    let rgb_img = img.to_rgb8();
    let resized_img = imageops::resize(&rgb_img, new_w, new_h, imageops::FilterType::Triangle);
    let mut canvas = RgbImage::from_pixel(target_width, target_height, image::Rgb([114, 114, 114]));
    imageops::overlay(&mut canvas, &resized_img, 0, 0);

    (canvas, new_w, new_h)
}

/// Takes raw image bytes, runs the detector, and returns a clean list of faces and the resize ratio.
pub fn detect_faces(
    session: &mut Session,
    image_bytes: &[u8],
    params: &DebugParams,
) -> Result<(Vec<DetectedFace>, u32, u32), AppError> {
    let image = image::load_from_memory(image_bytes)?;
    
    let (processed_img, new_w, new_h) =
        preprocess_image_topleft(&image, DETECTOR_TARGET_SHAPE.0, DETECTOR_TARGET_SHAPE.1);

    let mut input_tensor = Array::zeros((1, 3, DETECTOR_TARGET_SHAPE.0 as usize, DETECTOR_TARGET_SHAPE.1 as usize));
    for (x, y, pixel) in processed_img.enumerate_pixels() {
        input_tensor[[0, 0, y as usize, x as usize]] = (pixel[2] as f32 - 127.5) / 127.5;
        input_tensor[[0, 1, y as usize, x as usize]] = (pixel[1] as f32 - 127.5) / 127.5;
        input_tensor[[0, 2, y as usize, x as usize]] = (pixel[0] as f32 - 127.5) / 127.5;
    }

    let inputs = inputs!["input.1" => Value::from_array(input_tensor)?]?;
    let outputs = session.run(inputs)?;
    
    let score_8 = outputs["448"].try_extract_tensor::<f32>()?;
    let bbox_8 = outputs["451"].try_extract_tensor::<f32>()?;
    let kps_8 = outputs["454"].try_extract_tensor::<f32>()?;

    let score_16 = outputs["471"].try_extract_tensor::<f32>()?;
    let bbox_16 = outputs["474"].try_extract_tensor::<f32>()?;
    let kps_16 = outputs["477"].try_extract_tensor::<f32>()?;

    let score_32 = outputs["494"].try_extract_tensor::<f32>()?;
    let bbox_32 = outputs["497"].try_extract_tensor::<f32>()?;
    let kps_32 = outputs["500"].try_extract_tensor::<f32>()?;
    
    let all_outputs = [
        (8, score_8, bbox_8, kps_8),
        (16, score_16, bbox_16, kps_16),
        (32, score_32, bbox_32, kps_32),
    ];

    let proposals = decode_proposals(&all_outputs, DETECTOR_TARGET_SHAPE.1 as f32, DETECTOR_TARGET_SHAPE.0 as f32, params)?;

    let final_faces = non_maximum_suppression(&proposals, NMS_THRESHOLD);

    Ok((final_faces, new_w, new_h))
}

/// Decodes raw model output into candidate faces.
fn decode_proposals(
    outputs: &[(i32, ArrayBase<ViewRepr<&f32>, Dim<IxDynImpl>>, ArrayBase<ViewRepr<&f32>, Dim<IxDynImpl>>, ArrayBase<ViewRepr<&f32>, Dim<IxDynImpl>>); 3],
    img_width: f32,
    img_height: f32,
    params: &DebugParams,
) -> Result<Vec<DetectedFace>, AppError> {
    let conf_threshold = params.threshold.unwrap_or(0.7);
    let mut proposals = Vec::new();

    for (stride, scores_tuple, boxes, kps) in outputs {
        let scores = scores_tuple.slice(s![.., 0]);

        let feature_height = (img_height / *stride as f32).ceil() as usize;
        let feature_width = (img_width / *stride as f32).ceil() as usize;

        for y in 0..feature_height {
            for x in 0..feature_width {
                for anchor_idx in 0..2 {
                    let idx = y * feature_width * 2 + x * 2 + anchor_idx;
                    if idx >= scores.len() { continue; }
                    let score = scores[idx];

                    if score < conf_threshold { continue; }

                    let box_pred_arr = boxes.slice(s![idx as usize, ..]);
                    let box_pred = box_pred_arr.as_slice().unwrap();
                    let kps_pred_arr = kps.slice(s![idx as usize, ..]);
                    let kps_pred = kps_pred_arr.as_slice().unwrap();
                    let anchor_cx = (x as f32 + 0.5) * *stride as f32;
                    let anchor_cy = (y as f32 + 0.5) * *stride as f32;

                    let l = box_pred[0] * *stride as f32;
                    let t = box_pred[1] * *stride as f32;
                    let r = box_pred[2] * *stride as f32;
                    let b = box_pred[3] * *stride as f32;
                    let bbox = [anchor_cx - l, anchor_cy - t, anchor_cx + r, anchor_cy + b];

                    let mut decoded_kps = [[0.0; 2]; 5];
                    for k in 0..5 {
                        let kps_x = anchor_cx + kps_pred[k * 2] * *stride as f32;
                        let kps_y = anchor_cy + kps_pred[k * 2 + 1] * *stride as f32;
                        decoded_kps[k] = [kps_x, kps_y];
                    }
                    
                    proposals.push(DetectedFace { bbox, kps: decoded_kps, score });
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
    
    let mut keep_indices = Vec::new();
    let mut suppressed = vec![false; sorted_proposals.len()];

    for i in 0..sorted_proposals.len() {
        if suppressed[i] { continue; }
        keep_indices.push(i);

        for j in (i + 1)..sorted_proposals.len() {
            if suppressed[j] { continue; }
            let iou = calculate_iou(&sorted_proposals[i].bbox, &sorted_proposals[j].bbox);
            if iou > iou_threshold {
                suppressed[j] = true;
            }
        }
    }
    
    keep_indices.into_iter().map(|i| sorted_proposals[i].clone()).collect()
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
    
    if union_area <= 0.0 { 0.0 } else { intersection_area / union_area }
}

/// Takes a detected face, crops, and generates a 512-dim embedding.
pub fn get_recognition_embedding(
    session: &mut Session,
    original_image: &DynamicImage,
    face: &DetectedFace,
) -> Result<Vec<f32>, AppError> {
    let cropped_face = original_image.crop_imm(
        face.bbox[0].round() as u32,
        face.bbox[1].round() as u32,
        (face.bbox[2] - face.bbox[0]).round().max(0.0) as u32,
        (face.bbox[3] - face.bbox[1]).round().max(0.0) as u32,
    );

    let resized = cropped_face.resize_exact(
        RECOGNIZER_INPUT_SIZE,
        RECOGNIZER_INPUT_SIZE,
        image::imageops::FilterType::Triangle,
    );

    let mut input_tensor = Array::zeros((1, 3, RECOGNIZER_INPUT_SIZE as usize, RECOGNIZER_INPUT_SIZE as usize));
    for (x, y, pixel) in resized.to_rgb8().enumerate_pixels() {
        // Use BGR order for consistency
        input_tensor[[0, 0, y as usize, x as usize]] = (pixel[2] as f32 - 127.5) / 127.5;
        input_tensor[[0, 1, y as usize, x as usize]] = (pixel[1] as f32 - 127.5) / 127.5;
        input_tensor[[0, 2, y as usize, x as usize]] = (pixel[0] as f32 - 127.5) / 127.5;
    }

    let inputs = inputs!["input.1" => Value::from_array(input_tensor)?]?;
    let outputs = session.run(inputs)?;

    let data = outputs[0].try_extract_tensor::<f32>()?;
    let mut embedding: Vec<f32> = data.iter().cloned().collect();
    
    let norm = (embedding.iter().map(|v| v.powi(2)).sum::<f32>()).sqrt();
    if norm > 0.0 {
        embedding.iter_mut().for_each(|v| *v /= norm);
    }

    Ok(embedding)
}

/// Draws bounding boxes and keypoints on an image.
pub fn draw_detections(
    image: &mut DynamicImage,
    results: &[FinalResult],
    font: &FontArc,
) {
    debug!("Drawing {} detections on image", results.len());

    const THICKNESS: u32 = 3;
    // const DOT_RADIUS: i32 = 8;
    let box_color = Rgba([0u8, 0u8, 255u8, 255u8]);     // Blue
    // let dot_color = Rgba([255u8, 0u8, 0u8, 255u8]);     // Red
    let text_color = Rgba([255u8, 255u8, 255u8, 255u8]); // White (better contrast on blue)

    for result in results {
        let face = &result.detection;
        let x1 = face.bbox[0].round() as i32;
        let y1 = face.bbox[1].round() as i32;
        let x2 = face.bbox[2].round() as i32;
        let y2 = face.bbox[3].round() as i32;
        let width = (x2 - x1) as u32;
        
        // Draw Bounding Box (unchanged)
        for i in 0..THICKNESS {
            let rect = Rect::at(x1 + i as i32, y1 + i as i32)
                .of_size(width.saturating_sub(i * 2), (y2 - y1).saturating_sub(i as i32 * 2) as u32);
            draw_hollow_rect_mut(image, rect, box_color);
        }

        // Draw Keypoints (I've uncommented your code for this)
        // for point in face.kps {
        //     let center = (point[0].round() as i32, point[1].round() as i32);
        //     draw_filled_circle_mut(image, center, DOT_RADIUS, dot_color);
        // }

        // --- NEW: Draw Text Label with Background ---
        let text = match &result.recognition {
            Some((name, score)) => {
                if *score > 0.4 { // Only show label if similarity is decent
                    name.to_string()
                } else {
                    "Unknown".to_string()
                }
            },
            None => "Unknown".to_string(),
        };

        let font_scale = PxScale::from(32.0);
        
        // 1. Calculate the height of the text to size the background box
        let text_height = 32;
        let text_padding = 5; // Add some padding around the text

        // 2. Define the filled rectangle for the background
        let label_box_height = text_height + (text_padding * 2);
        let label_box_rect = Rect::at(x1, y2)
            .of_size(width, label_box_height);

        // 3. Draw the filled background box
        draw_filled_rect_mut(image, label_box_rect, box_color);
        
        // 4. Position and draw the text on top of the background
        let text_position = (x1 + text_padding as i32, y2 + text_padding as i32);
        draw_text_mut(image, text_color, text_position.0, text_position.1, font_scale, font, &text);
    }
}