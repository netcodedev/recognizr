import cv2
import numpy as np
import onnxruntime

# --- CONFIGURATION ---
CONF_THRESHOLD = 0.5
NMS_THRESHOLD = 0.4
INPUT_IMAGE_PATH = 'me.jpg'
MODEL_PATH = 'models/scrfd_10g_bnkps.onnx' 
TARGET_SHAPE = (640, 640) # (height, width)
# ---------------------

def preprocess_image_topleft(img, target_height, target_width):
    """
    Resizes an image to a target shape, maintaining aspect ratio,
    and pastes it at the top-left corner, padding the right and bottom.
    """
    img_h, img_w, _ = img.shape
    
    # Calculate the scaling ratio
    ratio = min(target_width / img_w, target_height / img_h)
    
    # Calculate new dimensions
    new_w, new_h = int(img_w * ratio), int(img_h * ratio)
    
    # Resize the image
    resized_img = cv2.resize(img, (new_w, new_h), interpolation=cv2.INTER_LINEAR)
    
    # Create a new canvas and paste the resized image at the top-left corner
    canvas = np.full((target_height, target_width, 3), 114, dtype=np.uint8)
    canvas[0:new_h, 0:new_w] = resized_img
    
    # This function now only needs to return the canvas and the ratio
    return canvas, ratio

def decode_proposals(outputs, img_width, img_height):
    """Decodes raw ONNX model output into proposals with bboxes and keypoints."""
    # This function is now correct and does not need changes.
    proposals = []
    strides = [8, 16, 32]
    
    for i, stride in enumerate(strides):
        scores = np.squeeze(outputs[i])
        boxes = np.squeeze(outputs[i+3])
        kps = np.squeeze(outputs[i+6])

        feature_height = int(np.ceil(img_height / stride))
        feature_width = int(np.ceil(img_width / stride))

        for y in range(feature_height):
            for x in range(feature_width):
                for anchor_idx in range(2):
                    idx = (y * feature_width * 2) + (x * 2) + anchor_idx
                    score = scores[idx]

                    if score < CONF_THRESHOLD:
                        continue

                    box_pred = boxes[idx]
                    anchor_cx = (x + 0.5) * stride
                    anchor_cy = (y + 0.5) * stride
                    
                    l, t, r, b = box_pred * stride
                    bbox = np.array([anchor_cx - l, anchor_cy - t, anchor_cx + r, anchor_cy + b])

                    kps_pred = kps[idx]
                    decoded_kps = np.zeros((5, 2))
                    for k in range(5):
                        kps_x = anchor_cx + kps_pred[k * 2] * stride
                        kps_y = anchor_cy + kps_pred[k * 2 + 1] * stride
                        decoded_kps[k] = [kps_x, kps_y]
                    
                    proposals.append({'bbox': bbox, 'score': score, 'kps': decoded_kps})
    return proposals

def nms(proposals):
    """Performs Non-Maximum Suppression on a list of proposals."""
    # This function is also correct and does not need changes.
    if not proposals: return []
    boxes = np.array([p['bbox'] for p in proposals])
    scores = np.array([p['score'] for p in proposals])
    order = scores.argsort()[::-1]
    keep_indices = []
    while order.size > 0:
        i = order[0]
        keep_indices.append(i)
        area = (boxes[i, 2] - boxes[i, 0]) * (boxes[i, 3] - boxes[i, 1])
        xx1 = np.maximum(boxes[i, 0], boxes[order[1:], 0])
        yy1 = np.maximum(boxes[i, 1], boxes[order[1:], 1])
        xx2 = np.minimum(boxes[i, 2], boxes[order[1:], 2])
        yy2 = np.minimum(boxes[i, 3], boxes[order[1:], 3])
        w = np.maximum(0.0, xx2 - xx1)
        h = np.maximum(0.0, yy2 - yy1)
        intersection = w * h
        union = area + (boxes[order[1:], 2] - boxes[order[1:], 0]) * (boxes[order[1:], 3] - boxes[order[1:], 1]) - intersection
        ovr = intersection / union
        inds = np.where(ovr <= NMS_THRESHOLD)[0]
        order = order[inds + 1]
    return [proposals[i] for i in keep_indices]

def main():
    """Main execution function."""
    session = onnxruntime.InferenceSession(MODEL_PATH, providers=['CPUExecutionProvider'])
    input_name = session.get_inputs()[0].name
    
    original_img = cv2.imread(INPUT_IMAGE_PATH)
    original_height, original_width = original_img.shape[:2]

    # Preprocess using the new top-left padding function
    processed_img, ratio = preprocess_image_topleft(original_img, TARGET_SHAPE[0], TARGET_SHAPE[1])
    
    input_tensor = (processed_img.astype(np.float32) - 127.5) / 127.5
    input_tensor = np.transpose(input_tensor, [2, 0, 1])
    input_tensor = np.expand_dims(input_tensor, axis=0)

    outputs = session.run(None, {input_name: input_tensor})
    
    proposals = decode_proposals(outputs, TARGET_SHAPE[1], TARGET_SHAPE[0])
    if not proposals: return print("Found 0 faces.")
        
    final_faces = nms(proposals)
    if not final_faces: return print("Found 0 faces after NMS.")

    print(f"Found {len(final_faces)} faces.")

    for face in final_faces:
        # --- FINAL COORDINATE SCALING LOGIC ---
        # Since we are not padding on the top or left, we just divide by the ratio.
        bbox = face['bbox'] / ratio
        kps = face['kps'] / ratio
        
        # Draw the final, correctly scaled box and points
        box = bbox.clip(0).astype(np.int32)
        cv2.rectangle(original_img, (box[0], box[1]), (box[2], box[3]), (0, 255, 0), 3)

        for k in range(kps.shape[0]):
            kp = kps[k].astype(np.int32)
            cv2.circle(original_img, tuple(kp), 8, (0, 0, 255), -1)
    
    cv2.imwrite('python_debug_output.jpg', original_img)
    print("Saved visualization to python_debug_output.jpg")

if __name__ == '__main__':
    main()