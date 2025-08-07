use crate::error::AppError;
use crate::models::{DebugParams, DetectedFace, FinalResult};
use crate::config::ModelMetadata;
use image::{imageops, DynamicImage, GenericImageView, RgbImage, Rgba};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use ndarray::{s, Array, ArrayBase, Dim, IxDynImpl, ViewRepr};
use ort::{inputs, session::{Session}, value::Value};
use ab_glyph::{FontArc, PxScale};
use tracing::debug;

// --- TUNING PARAMETERS ---
const NMS_THRESHOLD: f32 = 0.4;

// --- COORDINATE SCALING OFFSETS ---
// These offsets are applied during coordinate scaling to adjust for preprocessing differences
pub const X_OFFSET: f32 = 50.0;
pub const Y_OFFSET: f32 = 50.0;

// --- IMAGE PROCESSING CONSTANTS ---
const LETTERBOX_FILL_COLOR: [u8; 3] = [114, 114, 114]; // Gray color for letterbox padding
const NORMALIZATION_MEAN: f32 = 127.5;
const NORMALIZATION_SCALE: f32 = 127.5;

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
    let mut canvas = RgbImage::from_pixel(target_width, target_height, image::Rgb(LETTERBOX_FILL_COLOR));
    imageops::overlay(&mut canvas, &resized_img, 0, 0);

    (canvas, new_w, new_h)
}

/// Detects faces in an image using the SCRFD model.
///
/// # Arguments
/// * `session` - Mutable reference to the ONNX runtime session
/// * `image_bytes` - Raw image data as bytes
/// * `params` - Debug parameters for controlling detection behavior
/// * `detector_metadata` - Pre-computed model metadata with output mappings
///
/// # Returns
/// * `Ok((faces, width, height))` - List of detected faces and resized image dimensions
/// * `Err(AppError)` - If detection fails
///
/// # Performance
/// Uses pre-computed output mappings for efficient tensor extraction.
pub fn detect_faces(
    session: &mut Session,
    image_bytes: &[u8],
    params: &DebugParams,
    detector_metadata: &crate::config::DetectorMetadata,
) -> Result<(Vec<DetectedFace>, u32, u32), AppError> {
    let image = image::load_from_memory(image_bytes)?;

    // Extract target shape from detector metadata
    let target_height = detector_metadata.input_shape[2] as u32;
    let target_width = detector_metadata.input_shape[3] as u32;

    let (processed_img, new_w, new_h) =
        preprocess_image_topleft(&image, target_height, target_width);

    let mut input_tensor = Array::zeros((1, 3, target_height as usize, target_width as usize));
    for (x, y, pixel) in processed_img.enumerate_pixels() {
        // Normalize pixel values: (pixel - mean) / scale, using BGR order
        input_tensor[[0, 0, y as usize, x as usize]] = (pixel[2] as f32 - NORMALIZATION_MEAN) / NORMALIZATION_SCALE;
        input_tensor[[0, 1, y as usize, x as usize]] = (pixel[1] as f32 - NORMALIZATION_MEAN) / NORMALIZATION_SCALE;
        input_tensor[[0, 2, y as usize, x as usize]] = (pixel[0] as f32 - NORMALIZATION_MEAN) / NORMALIZATION_SCALE;
    }

    let inputs = inputs![&detector_metadata.input_name => Value::from_array(input_tensor)?]?;
    let outputs = session.run(inputs)?;
    
    // Use pre-computed output mappings to extract tensors efficiently
    let mut all_outputs = Vec::new();

    for (&stride, &(score_idx, bbox_idx, kps_idx)) in &detector_metadata.stride_output_mapping {
        let score_name = &detector_metadata.output_names[score_idx];
        let bbox_name = &detector_metadata.output_names[bbox_idx];
        let kps_name = &detector_metadata.output_names[kps_idx];

        let score = outputs[score_name.as_str()].try_extract_tensor::<f32>()?;
        let bbox = outputs[bbox_name.as_str()].try_extract_tensor::<f32>()?;
        let kps = outputs[kps_name.as_str()].try_extract_tensor::<f32>()?;

        all_outputs.push((stride, score, bbox, kps));
    }

    let proposals = decode_proposals(&all_outputs, target_width as f32, target_height as f32, params)?;

    let final_faces = non_maximum_suppression(&proposals, NMS_THRESHOLD);

    Ok((final_faces, new_w, new_h))
}

/// Decodes raw model output into candidate faces.
fn decode_proposals(
    outputs: &[(i32, ArrayBase<ViewRepr<&f32>, Dim<IxDynImpl>>, ArrayBase<ViewRepr<&f32>, Dim<IxDynImpl>>, ArrayBase<ViewRepr<&f32>, Dim<IxDynImpl>>)],
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
                    let expected_kps_len = 10; // 5 keypoints * 2 coordinates each

                    if kps_pred.len() >= expected_kps_len {
                        for k in 0..5 {
                            if k * 2 + 1 < kps_pred.len() {
                                let kps_x = anchor_cx + kps_pred[k * 2] * *stride as f32;
                                let kps_y = anchor_cy + kps_pred[k * 2 + 1] * *stride as f32;
                                decoded_kps[k] = [kps_x, kps_y];
                            } else {
                                tracing::warn!("Keypoint {} index out of bounds for kps_pred len {}", k, kps_pred.len());
                                break;
                            }
                        }
                    } else {
                        // If keypoints data is insufficient, use default values or skip
                        tracing::warn!("Insufficient keypoints data for stride {}: expected {}, got {}",
                                      stride, expected_kps_len, kps_pred.len());
                        // Keep default zeros for keypoints
                    }
                    
                    proposals.push(DetectedFace { bbox, kps: decoded_kps, score });
                }
            }
        }
    }
    Ok(proposals)
}

/// Pre-computes output mappings at startup for efficient runtime inference.
///
/// Runs the detector model once with dummy input to determine which outputs
/// correspond to scores, bounding boxes, and keypoints for each stride.
/// This eliminates the need for shape analysis during every inference.
///
/// # Arguments
/// * `session` - Mutable reference to the detector session
/// * `output_names` - List of model output names
/// * `strides` - List of detection strides (e.g., [8, 16, 32])
/// * `target_height` - Model input height
/// * `target_width` - Model input width
///
/// # Returns
/// * `Ok(HashMap)` - Mapping from stride to (score_idx, bbox_idx, kps_idx)
/// * `Err(AppError)` - If mapping computation fails
pub fn match_outputs_by_shape_at_startup(
    session: &mut Session,
    output_names: &[String],
    strides: &[i32],
    target_height: u32,
    target_width: u32,
) -> Result<std::collections::HashMap<i32, (usize, usize, usize)>, AppError> {
    use ndarray::Array4;
    use ort::value::Value;

    // Safety check: ensure dimensions are reasonable
    if target_height == 0 || target_width == 0 || target_height > 10000 || target_width > 10000 {
        return Err(AppError::BadRequest(format!(
            "Invalid target dimensions: {}x{}", target_width, target_height
        )));
    }

    // Create dummy input tensor
    let input_array = Array4::<f32>::zeros((1, 3, target_height as usize, target_width as usize));
    let input_tensor = Value::from_array(input_array)?;

    // Run inference to get output shapes
    let outputs = session.run(ort::inputs!["input.1" => input_tensor]?)?;

    // Extract all outputs with their shapes
    let mut extracted_outputs = Vec::new();
    for output_name in output_names {
        let tensor = outputs[output_name.as_str()].try_extract_tensor::<f32>()?;
        let shape = tensor.shape().to_vec();
        extracted_outputs.push((output_name.clone(), tensor, shape));
    }

    // Match outputs for each stride
    let mut stride_output_mapping = std::collections::HashMap::new();

    for &stride in strides {
        if let Some((score_idx, bbox_idx, kps_idx)) = match_outputs_by_shape(&extracted_outputs, stride, target_height, target_width)? {
            stride_output_mapping.insert(stride, (score_idx, bbox_idx, kps_idx));
        } else {
            return Err(AppError::BadRequest(format!("Could not find matching outputs for stride {}", stride)));
        }
    }

    if stride_output_mapping.is_empty() {
        return Err(AppError::BadRequest("No valid output mappings found for any stride".to_string()));
    }

    tracing::info!("Pre-computed output mappings for {} strides", stride_output_mapping.len());

    Ok(stride_output_mapping)
}

/// Match outputs by their shapes to determine which is score, bbox, and keypoints for a given stride
/// Returns indices into the extracted_outputs array
fn match_outputs_by_shape(
    extracted_outputs: &[(String, ArrayBase<ViewRepr<&f32>, Dim<IxDynImpl>>, Vec<usize>)],
    stride: i32,
    target_height: u32,
    target_width: u32,
) -> Result<Option<(usize, usize, usize)>, AppError> {

    // Calculate expected number of anchors for this stride
    // SCRFD typically uses 2 anchors per spatial location
    let feat_h = target_height / stride as u32;
    let feat_w = target_width / stride as u32;
    let num_anchors_per_location = 2;
    let expected_total_anchors = feat_h * feat_w * num_anchors_per_location;



    let mut score_idx = None;
    let mut bbox_idx = None;
    let mut kps_idx = None;

    // Look for outputs that match the expected flattened shapes for this stride
    for (idx, (_name, _tensor, shape)) in extracted_outputs.iter().enumerate() {
        if shape.len() == 2 {
            let num_elements = shape[0];
            let channels = shape[1];

            // Check if this output corresponds to our stride's expected anchor count
            if num_elements == expected_total_anchors as usize {
                // Classify based on channel count
                match channels {
                    1 => {
                        // Score output (1 channel for face/no-face)
                        if score_idx.is_none() {
                            score_idx = Some(idx);
                        }
                    },
                    4 => {
                        // Bbox output (4 channels for x, y, w, h)
                        if bbox_idx.is_none() {
                            bbox_idx = Some(idx);
                        }
                    },
                    10 => {
                        // Keypoints output (10 channels for 5 keypoints * 2 coordinates)
                        if kps_idx.is_none() {
                            kps_idx = Some(idx);
                        }
                    },
                    _ => {
                        // Unexpected channel count, skip
                    }
                }
            }
        }
    }

    // Return the matched indices if we found all three
    if let (Some(score), Some(bbox), Some(kps)) = (score_idx, bbox_idx, kps_idx) {
        Ok(Some((score, bbox, kps)))
    } else {
        Ok(None)
    }
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
    recognizer_metadata: &ModelMetadata,
) -> Result<Vec<f32>, AppError> {
    let (image_width, image_height) = original_image.dimensions();
    let (x, y, width, height) = face.get_safe_crop_coords(image_width, image_height);

    let cropped_face = original_image.crop_imm(x, y, width, height);

    // Extract input size from recognizer metadata
    let input_size = recognizer_metadata.input_shape[2] as u32; // Assuming square input

    let resized = cropped_face.resize_exact(
        input_size,
        input_size,
        image::imageops::FilterType::Triangle,
    );

    let mut input_tensor = Array::zeros((1, 3, input_size as usize, input_size as usize));
    for (x, y, pixel) in resized.to_rgb8().enumerate_pixels() {
        // Normalize pixel values: (pixel - mean) / scale, using BGR order for consistency
        input_tensor[[0, 0, y as usize, x as usize]] = (pixel[2] as f32 - NORMALIZATION_MEAN) / NORMALIZATION_SCALE;
        input_tensor[[0, 1, y as usize, x as usize]] = (pixel[1] as f32 - NORMALIZATION_MEAN) / NORMALIZATION_SCALE;
        input_tensor[[0, 2, y as usize, x as usize]] = (pixel[0] as f32 - NORMALIZATION_MEAN) / NORMALIZATION_SCALE;
    }

    let inputs = inputs![&recognizer_metadata.input_name => Value::from_array(input_tensor)?]?;
    let outputs = session.run(inputs)?;

    let output_name = &recognizer_metadata.output_names[0];
    let data = outputs[output_name.as_str()].try_extract_tensor::<f32>()?;
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