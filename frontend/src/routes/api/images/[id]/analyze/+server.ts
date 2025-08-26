import { json } from '@sveltejs/kit';
import { readFile } from 'fs/promises';
import { existsSync } from 'fs';
import path from 'path';
import type { RequestHandler } from './$types';
import type { SavedImage } from '../../+server';
import { API_BASE } from '$lib/api';

const IMAGES_DIR = 'static/images';
const METADATA_FILE = path.join(IMAGES_DIR, 'metadata.json');

// Load metadata from file
async function loadMetadata(): Promise<SavedImage[]> {
    try {
        if (!existsSync(METADATA_FILE)) {
            return [];
        }
        const content = await readFile(METADATA_FILE, 'utf-8');
        return JSON.parse(content);
    } catch (error) {
        console.error('Failed to load metadata:', error);
        return [];
    }
}

// POST /api/images/[id]/analyze - Analyze faces in a saved image
export const POST: RequestHandler = async ({ params }) => {
    try {
        const imageId = params.id;
        
        if (!imageId) {
            return json({ error: 'Missing image ID' }, { status: 400 });
        }

        // Find the saved image in metadata
        const images = await loadMetadata();
        const savedImage = images.find(img => img.id === imageId);
        
        if (!savedImage) {
            return json({ error: 'Image not found' }, { status: 404 });
        }

        // Check if the image file exists
        if (!existsSync(savedImage.file_path)) {
            return json({ error: 'Image file not found on disk' }, { status: 404 });
        }

        // Read the image file
        const imageBuffer = await readFile(savedImage.file_path);

        // Create a File object to send to the Recognizr API
        const imageBlob = new Blob([imageBuffer]);
        const formData = new FormData();
        formData.append('image', imageBlob, savedImage.original_filename);

        // Call the Recognizr API
        const response = await fetch(`${API_BASE}/recognize`, {
            method: 'POST',
            body: formData
        });

        if (!response.ok) {
            const errorText = await response.text();
            return json({ 
                error: `Face recognition failed: ${errorText}` 
            }, { status: response.status });
        }

        const results = await response.json();
        return json(results);
    } catch (error) {
        console.error('Failed to analyze image:', error);
        return json({ error: 'Failed to analyze image' }, { status: 500 });
    }
};
