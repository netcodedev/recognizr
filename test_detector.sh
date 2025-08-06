#!/bin/bash

# --- Default Configuration ---
# These values will be used if not provided via command-line flags.
DEFAULT_IMAGE_IN="me.jpg"
DEFAULT_IMAGE_OUT="debug_output.png"
DEFAULT_HOST="http://localhost:3000"
DEFAULT_THRESHOLD="0.7"
# ---------------------

# --- Help/Usage Function ---
usage() {
    echo "Usage: $0 [-i <input_image>] [-o <output_image>] [-t <threshold>] [-u <host_url>] [-h]"
    echo ""
    echo "Sends an image to the face recognition debug endpoint and saves the visual result."
    echo ""
    echo "Options:"
    echo "  -i <path>    Path to the input image. (Default: $DEFAULT_IMAGE_IN)"
    echo "  -o <path>    Path to save the output image. (Default: $DEFAULT_IMAGE_OUT)"
    echo "  -t <float>   Confidence threshold for face detection (e.g., 0.5). (Default: $DEFAULT_THRESHOLD)"
    echo "  -u <url>     URL of the running application. (Default: $DEFAULT_HOST)"
    echo "  -h           Display this help message."
    exit 1
}

# --- Initialize variables with defaults ---
IMAGE_IN="$DEFAULT_IMAGE_IN"
IMAGE_OUT="$DEFAULT_IMAGE_OUT"
HOST="$DEFAULT_HOST"
THRESHOLD="$DEFAULT_THRESHOLD"

# --- Parse Command-Line Arguments ---
while getopts ":i:o:t:u:h" opt; do
    case ${opt} in
        i)
            IMAGE_IN="$OPTARG"
            ;;
        o)
            IMAGE_OUT="$OPTARG"
            ;;
        t)
            THRESHOLD="$OPTARG"
            ;;
        u)
            HOST="$OPTARG"
            ;;
        h)
            usage
            ;;
        \?)
            echo "Invalid Option: -$OPTARG" 1>&2
            usage
            ;;
        :)
            echo "Invalid Option: -$OPTARG requires an argument" 1>&2
            usage
            ;;
    esac
done

# --- Pre-flight Checks ---
# Check if curl is installed
if ! command -v curl &> /dev/null; then
    echo "Error: curl is not installed. Please install it to run this script."
    exit 1
fi

# Check if the input image file exists
if [ ! -f "$IMAGE_IN" ]; then
    echo "Error: Input image file not found at '$IMAGE_IN'"
    exit 1
fi

# --- Execution ---
# Construct the full URL
URL="$HOST/debug/detector?threshold=$THRESHOLD"

echo "--- Starting Face Detector Test ---"
echo "Host:         $HOST"
echo "Input Image:  $IMAGE_IN"
echo "Output Image: $IMAGE_OUT"
echo "Threshold:    $THRESHOLD"
echo "-----------------------------------"
echo ""

echo "Running request to: $URL"
time curl -s -X POST "$URL" \
  -F "image=@$IMAGE_IN" \
  --output "$IMAGE_OUT"

# Check if curl was successful
if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Success: Saved debug image to '$IMAGE_OUT'"
else
    echo ""
    echo "❌ Error: curl command failed. Is the server running at $HOST?"
fi