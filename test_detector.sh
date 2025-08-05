#!/bin/bash

# --- Configuration ---
# Make sure the image file specified here exists in the same directory as the script.
IMAGE_FILE="me.jpg"
# The base URL for your running application.
HOST="http://localhost:3000"
# The threshold to test with.
THRESHOLD="0.2"
# ---------------------

# Check if the image file exists before starting.
if [ ! -f "$IMAGE_FILE" ]; then
    echo "Error: Image file not found at '$IMAGE_FILE'"
    exit 1
fi

echo "--- Starting Face Detector Test ---"
echo "Using image: $IMAGE_FILE"
echo "Using threshold: $THRESHOLD"
echo ""

# Test 1: RGB with "ltrb" decoding
echo "Running Test 1: RGB color order, LTRB decoding..."
curl -s -X POST "$HOST/debug/detector?threshold=$THRESHOLD&order=rgb&decode=ltrb" \
  -F "image=@$IMAGE_FILE" \
  --output debug_rgb_ltrb.png
echo "  -> Saved to debug_rgb_ltrb.png"
echo ""

# Test 2: BGR with "ltrb" decoding
echo "Running Test 2: BGR color order, LTRB decoding..."
curl -s -X POST "$HOST/debug/detector?threshold=$THRESHOLD&order=bgr&decode=ltrb" \
  -F "image=@$IMAGE_FILE" \
  --output debug_bgr_ltrb.png
echo "  -> Saved to debug_bgr_ltrb.png"
echo ""

# Test 3: RGB with "center_wh" decoding
echo "Running Test 3: RGB color order, Center/WH decoding..."
curl -s -X POST "$HOST/debug/detector?threshold=$THRESHOLD&order=rgb&decode=center_wh" \
  -F "image=@$IMAGE_FILE" \
  --output debug_rgb_center.png
echo "  -> Saved to debug_rgb_center.png"
echo ""

# Test 4: BGR with "center_wh" decoding
echo "Running Test 4: BGR color order, Center/WH decoding..."
curl -s -X POST "$HOST/debug/detector?threshold=$THRESHOLD&order=bgr&decode=center_wh" \
  -F "image=@$IMAGE_FILE" \
  --output debug_bgr_center.png
echo "  -> Saved to debug_bgr_center.png"
echo ""

echo "--- Test Complete ---"
echo "Please inspect the four 'debug_*.png' images to find the correct combination."