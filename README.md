# Recognizr: A High-Performance Face Recognition API

**Recognizr** is a self-hosted, high-performance API for face detection and recognition, written entirely in Rust. It leverages the ONNX Runtime for efficient AI model inference, with optional GPU acceleration via CUDA.

This project was built to be a robust foundation for various computer vision tasks, providing endpoints for enrolling new individuals, recognizing them from images, and visually debugging the detection process.

## Features

* **Fast Inference:** Built in Rust with the `ort` crate, leveraging the high-performance ONNX Runtime.
* **GPU Acceleration:** Optional support for NVIDIA GPUs via CUDA for a >15x performance increase.
* **State-of-the-Art Models:** Uses powerful, pre-trained models for face detection (SCRFD) and recognition (InsightFace ArcFace R100).
* **Automatic Model Adaptation:** Dynamically detects model output structure at startup for compatibility with different model variants.
* **Optimized Performance:** Pre-computed output mappings eliminate shape analysis overhead during inference.
* **Vector Search:** Utilizes SurrealDB for efficient and scalable similarity searches on face embeddings.
* **Robust API:** Built with `axum`, providing endpoints for enrolling, recognizing, and debugging with comprehensive input validation.
* **Advanced Debugging:** A dedicated debug endpoint that visually renders detection boxes, keypoints, and recognition labels on an image.

## Setup and Installation

### 1. Prerequisites

* **Rust Toolchain:** Ensure you have Rust and Cargo installed. ([rustup.rs](https://rustup.rs))
* **(Optional) GPU Support:** To enable GPU acceleration, you must have a compatible NVIDIA GPU and the required drivers and libraries installed on your system.

#### For Arch Linux

```bash
# Install the NVIDIA driver, CUDA Toolkit, and cuDNN library
sudo pacman -Syu nvidia cuda cudnn
# A reboot is required after driver installation.
```

#### For Debian / Ubuntu

```bash
# Install the NVIDIA driver (this command selects the best recommended driver)
sudo ubuntu-drivers autoinstall
# A reboot is required after driver installation.

# Install the CUDA Toolkit and cuDNN from NVIDIA's official repository
# (Commands may vary slightly based on specific OS version - see NVIDIA's documentation)
sudo apt-get update
sudo apt-get install -y build-essential
# Download and install the CUDA repo pin and .deb package, then the toolkit
wget [https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.1-1_all.deb](https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.1-1_all.deb)
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt-get update
sudo apt-get -y install cuda-toolkit-12-5 libcudnn9-cuda-12-dev
```

### 2. Configuration

Recognizr uses a configuration file to manage all settings including file paths, database connection, and server configuration.

1. **Configuration File:** Create or modify the `config.toml` file in the project root. A default configuration file is provided with sensible defaults.

    ```toml
    # Recognizr Configuration File

    [font]
    path = "DejaVuSansMono.ttf"

    [models.detector]
    path = "models/scrfd_10g_bnkps.onnx"
    strides = [8, 16, 32]
    input_shape = [640, 640]  # [height, width]

    [models.recognizer]
    path = "models/arcface_r100.onnx"
    input_size = 112

    [database]
    host = "127.0.0.1"
    port = 8000
    username = "root"
    password = "root"
    namespace = "test"
    database = "test"

    [server]
    host = "0.0.0.0"
    port = 3000
    ```

2. **Environment Variable Overrides:** You can override any configuration setting using environment variables with the `RECOGNIZR_` prefix:

    ```bash
    # Override database host
    export RECOGNIZR_DATABASE_HOST=192.168.1.100

    # Override server port
    export RECOGNIZR_SERVER_PORT=8080

    # Override model paths and settings
    export RECOGNIZR_MODELS_DETECTOR_PATH=/custom/path/detector.onnx
    export RECOGNIZR_MODELS_DETECTOR_INPUT_SHAPE="[512,512]"
    export RECOGNIZR_MODELS_RECOGNIZER_INPUT_SIZE=112
    ```

### 3. Required Assets & Models

Before running the application, you must download and place the required model and font files as specified in your configuration.

1. **Create the `models` Directory:** In your project root, create a `models` directory (or the directory specified in your config).

    ```bash
    mkdir models
    ```

2. **Download Models:** You need two ONNX models. Place them in the paths specified in your configuration (default: `/models` directory).
    * **Face Detector:** `scrfd_10g_bnkps.onnx`
    * **Face Recognizer:** `arcface_r100.onnx` (or another compatible InsightFace recognition model)
3. **Download Font:** The debug endpoint requires a font file for drawing text. Download and place it in the path specified in your configuration (default: root directory).
    * **Font File:** `DejaVuSansMono.ttf`

After this step, your directory structure should look like this:

```text
recognizr/
├── models/
│   ├── scrfd_10g_bnkps.onnx
│   └── arcface_r100.onnx
├── src/
│   └── ...
├── Cargo.toml
├── config.toml
└── DejaVuSansMono.ttf
```

### 4. Building the Application

Build the application in release mode for the best performance. If you have set up GPU support, ensure the `cuda` feature is enabled in your `Cargo.toml` for the `ort` crate.

```bash
cargo build --release
```

## Running the Application

The application will load its configuration from `config.toml` in the project root. You can override any configuration setting using environment variables with the `RECOGNIZR_` prefix.

To run with GPU acceleration, you must set the LD_LIBRARY_PATH environment variable so the application can find the necessary ONNX and CUDA library files at runtime.

1. **Find the ONNX Runtime Library Path:** This library is located inside your project's target directory after building. Find its parent directory with this command:

    ```bash
    ORT_LIB_PATH=$(dirname $(find ./target/release -name "libonnxruntime_providers_shared.so"))
    ```

2. **Run the Server:** Combine the library path with the system's CUDA path and run the application.

### Running on Arch Linux

```bash
export LD_LIBRARY_PATH=$ORT_LIB_PATH:/opt/cuda/lib:$LD_LIBRARY_PATH
RUST_LOG=recognizr=info ./target/release/recognizr
```

### Running on Debian / Ubuntu

The default CUDA path is different.

```bash
export LD_LIBRARY_PATH=$ORT_LIB_PATH:/usr/local/cuda/lib64:$LD_LIBRARY_PATH
RUST_LOG=recognizr=info ./target/release/recognizr
```

The server will start on the address specified in your configuration (default: [http://localhost:3000](http://localhost:3000)).

## API Usage

`POST /enroll`
Enrolls a single person by detecting their face and storing its biometric embedding in the database. The image should contain exactly one face.

### /enroll Form Data

* `name`: `string` - The name of the person to enroll.
* `image`: `file` - The image file containing the person's face.

Example:

```bash
curl -X POST http://localhost:3000/enroll \
-F "name=Ada Lovelace" \
-F "image=@/path/to/ada.jpg"
```

Response: `201 Created` on success.

`POST /recognize`
Finds and recognizes all known faces in a given image.

### /recognizeForm Data

* `image`: `file` - The image file to be analyzed.

Example:

```bash
curl -X POST http://localhost:3000/recognize \
-F "image=@/path/to/group_photo.jpg"
```

Example Success Response:

```json
[
    {
        "name": "Ada Lovelace",
        "similarity_score": 0.87,
        "bbox": [
        150.5,
        210.2,
        390.8,
        505.1
        ]
    }
]
```

`POST /debug/detector`
A powerful debug endpoint that runs the full detection and recognition pipeline on an image and returns a new image with the results visually rendered.

### /debug/detector Form Data

* `image`: `file` - The image file to be analyzed.

Query Parameters (Optional):

* `threshold`: `float` - Overrides the default confidence threshold for face detection (e.g., ?threshold=0.6).

Example:

```bash
curl -X POST "http://localhost:3000/debug/detector?threshold=0.75" \
-F "image=@/path/to/my_photo.jpg" \
--output debug_result.jpg
```

Response: An image/jpeg or image/png file with bounding boxes, keypoints, and labels drawn on it.

## Configuration Management

Recognizr uses a flexible configuration system that supports both file-based configuration and environment variable overrides.

### Configuration File Structure

The `config.toml` file contains all application settings organized into logical sections:

* **`[font]`** - Font file configuration for debug rendering
* **`[models.detector]`** - Face detector model configuration
* **`[models.recognizer]`** - Face recognizer model configuration
* **`[database]`** - SurrealDB connection settings
* **`[server]`** - HTTP server configuration

### Model Configuration

Recognizr automatically detects model outputs by analyzing their shapes at startup, but requires some configuration to work with different model architectures:

#### Detector Configuration (`[models.detector]`)

* **`path`** - Path to the ONNX detector model file
* **`strides`** - Detection strides used by the model (typically `[8, 16, 32]` for SCRFD)
* **`input_shape`** - Model input dimensions as `[height, width]` (e.g., `[640, 640]`)

#### Recognizer Configuration (`[models.recognizer]`)

* **`path`** - Path to the ONNX recognizer model file
* **`input_size`** - Square input size for face crops (e.g., `112` for 112x112 input)

#### Automatic Output Detection

The system automatically:

1. **Analyzes model outputs** at startup by running inference once with dummy input
2. **Matches outputs by shape** to determine which correspond to scores, bounding boxes, and keypoints
3. **Pre-computes mappings** for efficient runtime inference
4. **Supports different model architectures** as long as they follow the SCRFD output pattern

This means you can use different SCRFD variants or input sizes without manual output mapping configuration.

### Environment Variable Overrides

Any configuration setting can be overridden using environment variables with the `RECOGNIZR_` prefix. The variable names follow the pattern `RECOGNIZR_<SECTION>_<SETTING>`.

Examples:

```bash
# Database configuration
export RECOGNIZR_DATABASE_HOST=192.168.1.100
export RECOGNIZR_DATABASE_PORT=8001
export RECOGNIZR_DATABASE_USERNAME=myuser
export RECOGNIZR_DATABASE_PASSWORD=mypassword

# Server configuration
export RECOGNIZR_SERVER_HOST=127.0.0.1
export RECOGNIZR_SERVER_PORT=8080

# Detector model configuration
export RECOGNIZR_MODELS_DETECTOR_PATH=/custom/models/detector.onnx
export RECOGNIZR_MODELS_DETECTOR_STRIDES="[8,16,32]"
export RECOGNIZR_MODELS_DETECTOR_INPUT_SHAPE="[512,512]"

# Recognizer model configuration
export RECOGNIZR_MODELS_RECOGNIZER_PATH=/custom/models/recognizer.onnx
export RECOGNIZR_MODELS_RECOGNIZER_INPUT_SIZE=112

# Font path
export RECOGNIZR_FONT_PATH=/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf
```

This makes it easy to deploy Recognizr in different environments (development, staging, production) without modifying the configuration file.

## Input Validation

Recognizr includes comprehensive input validation to ensure robust operation:

### Image Validation

* **File size**: Maximum 15MB per image
* **Dimensions**: Minimum 32x32 pixels, maximum 8192x8192 pixels
* **Format**: Supports common image formats (JPEG, PNG, etc.)

### Name Validation (for enrollment)

* **Length**: Maximum 100 characters
* **Content**: Cannot be empty or whitespace-only

These limits help prevent resource exhaustion and ensure consistent performance across different deployment environments.
