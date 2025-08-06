#!/bin/bash

# --- Configuration ---
# Make sure the image file specified here exists in the same directory as the script.
IMAGE_FILE="me.jpg"
# The base URL for your running application.
HOST="http://localhost:3000"
# The threshold to test with.
THRESHOLD="0.7"
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
echo "Running Test"
time curl -s -X POST "$HOST/debug/detector?threshold=$THRESHOLD" \
  -F "image=@$IMAGE_FILE" \
  --output debug_image.png
echo "  -> Saved to debug_image.png"