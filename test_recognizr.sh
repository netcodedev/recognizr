#!/bin/bash

# --- Default Configuration ---
# These values will be used if not provided via command-line flags.
DEFAULT_IMAGE_IN="me.jpg"
DEFAULT_IMAGE_OUT="debug_output.png"
DEFAULT_HOST="http://localhost:3000"
DEFAULT_THRESHOLD="0.7"
DEFAULT_MODE="debug"
# ---------------------

# --- Help/Usage Function ---
usage() {
    echo "Usage: $0 -m <mode> [-i <input_image>] [-o <output_image>] [-n <name>] [-t <threshold>] [-u <host_url>] [-h]"
    echo ""
    echo "Face recognition API testing tool supporting enroll, recognize, and debug endpoints."
    echo ""
    echo "Required Options:"
    echo "  -m <mode>    Operation mode: 'enroll', 'recognize', or 'debug'"
    echo ""
    echo "Optional Options:"
    echo "  -i <path>    Path to the input image. (Default: $DEFAULT_IMAGE_IN)"
    echo "  -o <path>    Path to save the output image (debug mode only). (Default: $DEFAULT_IMAGE_OUT)"
    echo "  -n <name>    Name of the person (enroll mode only)."
    echo "  -t <float>   Confidence threshold for face detection (debug mode only). (Default: $DEFAULT_THRESHOLD)"
    echo "  -u <url>     URL of the running application. (Default: $DEFAULT_HOST)"
    echo "  -h           Display this help message."
    echo ""
    echo "Mode-specific usage:"
    echo "  Enroll:     $0 -m enroll -i person.jpg -n \"John Doe\""
    echo "  Recognize:  $0 -m recognize -i group_photo.jpg"
    echo "  Debug:      $0 -m debug -i photo.jpg -o debug_result.png [-t 0.7]"
    exit 1
}

# --- Initialize variables with defaults ---
IMAGE_IN="$DEFAULT_IMAGE_IN"
IMAGE_OUT="$DEFAULT_IMAGE_OUT"
HOST="$DEFAULT_HOST"
THRESHOLD="$DEFAULT_THRESHOLD"
MODE="$DEFAULT_MODE"
NAME=""

# --- Parse Command-Line Arguments ---
while getopts ":m:i:o:n:t:u:h" opt; do
    case ${opt} in
        m)
            MODE="$OPTARG"
            ;;
        i)
            IMAGE_IN="$OPTARG"
            ;;
        o)
            IMAGE_OUT="$OPTARG"
            ;;
        n)
            NAME="$OPTARG"
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

# --- Validation ---
# Validate mode
case "$MODE" in
    "enroll"|"recognize"|"debug")
        ;;
    *)
        echo "Error: Invalid mode '$MODE'. Must be 'enroll', 'recognize', or 'debug'."
        usage
        ;;
esac

# Mode-specific validation
if [ "$MODE" = "enroll" ] && [ -z "$NAME" ]; then
    echo "Error: Name (-n) is required for enroll mode."
    usage
fi

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

# --- Execution Functions ---
run_enroll() {
    local url="$HOST/enroll"

    echo "--- Starting Face Enrollment ---"
    echo "Host:        $HOST"
    echo "Input Image: $IMAGE_IN"
    echo "Person Name: $NAME"
    echo "--------------------------------"
    echo ""

    echo "Running enrollment request to: $url"
    local response
    response=$(curl -s -w "\n%{http_code}" -X POST "$url" \
        -F "name=$NAME" \
        -F "image=@$IMAGE_IN")

    local http_code=$(echo "$response" | tail -n1)
    local body=$(echo "$response" | head -n -1)

    if [ "$http_code" = "201" ]; then
        echo ""
        echo "✅ Success: '$NAME' has been enrolled successfully!"
    else
        echo ""
        echo "❌ Error: Enrollment failed (HTTP $http_code)"
        if [ -n "$body" ]; then
            echo "Response: $body"
        fi
        exit 1
    fi
}

run_recognize() {
    local url="$HOST/recognize"

    echo "--- Starting Face Recognition ---"
    echo "Host:        $HOST"
    echo "Input Image: $IMAGE_IN"
    echo "---------------------------------"
    echo ""

    echo "Running recognition request to: $url"
    local response
    response=$(curl -s -w "\n%{http_code}" -X POST "$url" \
        -F "image=@$IMAGE_IN")

    local http_code=$(echo "$response" | tail -n1)
    local body=$(echo "$response" | head -n -1)

    if [ "$http_code" = "200" ]; then
        echo ""
        echo "✅ Success: Recognition completed!"
        echo "Results:"
        echo "$body" | python3 -m json.tool 2>/dev/null || echo "$body"
    else
        echo ""
        echo "❌ Error: Recognition failed (HTTP $http_code)"
        if [ -n "$body" ]; then
            echo "Response: $body"
        fi
        exit 1
    fi
}

run_debug() {
    local url="$HOST/debug/detector?threshold=$THRESHOLD"

    echo "--- Starting Face Debug Analysis ---"
    echo "Host:         $HOST"
    echo "Input Image:  $IMAGE_IN"
    echo "Output Image: $IMAGE_OUT"
    echo "Threshold:    $THRESHOLD"
    echo "------------------------------------"
    echo ""

    echo "Running debug request to: $url"
    time curl -s -X POST "$url" \
        -F "image=@$IMAGE_IN" \
        --output "$IMAGE_OUT"

    # Check if curl was successful
    if [ $? -eq 0 ]; then
        echo ""
        echo "✅ Success: Saved debug image to '$IMAGE_OUT'"
    else
        echo ""
        echo "❌ Error: curl command failed. Is the server running at $HOST?"
        exit 1
    fi
}

# --- Main Execution ---
case "$MODE" in
    "enroll")
        run_enroll
        ;;
    "recognize")
        run_recognize
        ;;
    "debug")
        run_debug
        ;;
esac