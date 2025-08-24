# Recognizr Frontend

A SvelteKit-based web application for the Recognizr face recognition system.

## Features

- **Enroll Person**: Add new people to the face recognition database
- **Recognize Faces**: Identify known faces in uploaded images
- **Debug Detection**: Visualize face detection with annotated images
- **Real-time Validation**: Client-side image validation and error handling
- **Responsive Design**: Works on desktop and mobile devices

## Prerequisites

- Node.js 18+ and pnpm (or npm/yarn)
- Recognizr API server running on `http://localhost:3000`

## Getting Started

1. **Install dependencies:**
   ```bash
   pnpm install
   ```

2. **Start the development server:**
   ```bash
   pnpm dev
   ```

3. **Open your browser:**
   Navigate to `http://localhost:5173`

## Usage

### Enrolling a Person

1. Click on the "Enroll Person" tab
2. Enter the person's name
3. Select an image file containing exactly one face
4. Click "Enroll Person"

**Requirements:**
- Image must contain exactly one face
- Supported formats: JPG, PNG, WebP
- Maximum file size: 15MB
- Name must be 1-100 characters

### Recognizing Faces

1. Click on the "Recognize Faces" tab
2. Select an image file that may contain multiple faces
3. Click "Recognize Faces"
4. View the results showing detected faces with names and confidence scores

**Results include:**
- Person's name (or "Unknown" if not recognized)
- Similarity percentage
- Confidence level (High/Medium/Low)
- Bounding box coordinates

### Debug Detection

1. Click on the "Debug Detection" tab
2. Adjust the detection threshold (0.1 - 1.0)
3. Select an image file
4. Click "Generate Debug Image"
5. View the annotated image with face detection boxes and labels
6. Download the debug image if needed

## API Configuration

The frontend is configured to connect to the Recognizr API at `http://localhost:3000`.

To change the API endpoint, modify the `API_BASE` constant in `src/lib/api.ts`.

## Building

To create a production version of your app:

```bash
pnpm build
```

You can preview the production build with `pnpm preview`.

## Technologies Used

- **SvelteKit**: Full-stack web framework
- **TypeScript**: Type-safe JavaScript
- **Tailwind CSS**: Utility-first CSS framework
- **Vite**: Fast build tool and dev server
