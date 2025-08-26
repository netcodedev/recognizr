import { json } from '@sveltejs/kit';
import { writeFile, readdir, mkdir } from 'fs/promises';
import { existsSync } from 'fs';
import path from 'path';
import type { RequestHandler } from './$types';

// Types for saved images
export interface SavedImage {
    id: string;
    filename: string;
    original_filename: string;
    file_path: string;
    created_at: string;
    source: string;
}

const IMAGES_DIR = 'static/images';
const METADATA_FILE = path.join(IMAGES_DIR, 'metadata.json');

// Ensure images directory exists
async function ensureImagesDir() {
    if (!existsSync(IMAGES_DIR)) {
        await mkdir(IMAGES_DIR, { recursive: true });
    }
}

// Load metadata from file
async function loadMetadata(): Promise<SavedImage[]> {
    try {
        if (!existsSync(METADATA_FILE)) {
            return [];
        }
        const data = await readdir(IMAGES_DIR);
        const metadataExists = data.includes('metadata.json');
        if (!metadataExists) {
            return [];
        }
        
        const { readFile } = await import('fs/promises');
        const content = await readFile(METADATA_FILE, 'utf-8');
        return JSON.parse(content);
    } catch (error) {
        console.error('Failed to load metadata:', error);
        return [];
    }
}

// Save metadata to file
async function saveMetadata(images: SavedImage[]) {
    try {
        await ensureImagesDir();
        await writeFile(METADATA_FILE, JSON.stringify(images, null, 2));
    } catch (error) {
        console.error('Failed to save metadata:', error);
        throw error;
    }
}

// GET /api/images - List all saved images
export const GET: RequestHandler = async () => {
    try {
        const images = await loadMetadata();
        return json(images);
    } catch (error) {
        console.error('Failed to list images:', error);
        return json({ error: 'Failed to list images' }, { status: 500 });
    }
};

// POST /api/images - Download and save a Google Photos image
export const POST: RequestHandler = async ({ request }) => {
    try {
        const { media_item, access_token } = await request.json();
        
        if (!media_item || !access_token) {
            return json({ error: 'Missing media_item or access_token' }, { status: 400 });
        }

        // Download the image from Google Photos
        const downloadUrl = `${media_item.mediaFile.baseUrl}=d`;
        const response = await fetch(downloadUrl, {
            headers: {
                'Authorization': `Bearer ${access_token}`
            }
        });

        if (!response.ok) {
            return json({ 
                error: `Failed to download image: HTTP ${response.status}` 
            }, { status: 400 });
        }

        const imageBuffer = await response.arrayBuffer();
        
        // Generate unique filename
        const id = crypto.randomUUID();
        const originalFilename = media_item.filename || `photo_${media_item.id}.jpg`;
        const extension = path.extname(originalFilename) || '.jpg';
        const filename = `${id}${extension}`;
        const filePath = path.join(IMAGES_DIR, filename);

        // Ensure directory exists and save the image
        await ensureImagesDir();
        await writeFile(filePath, new Uint8Array(imageBuffer));

        // Create metadata entry
        const savedImage: SavedImage = {
            id,
            filename,
            original_filename: originalFilename,
            file_path: filePath,
            created_at: new Date().toISOString(),
            source: 'google_photos'
        };

        // Load existing metadata, add new image, and save
        const images = await loadMetadata();
        images.push(savedImage);
        await saveMetadata(images);

        return json(savedImage);
    } catch (error) {
        console.error('Failed to download and save image:', error);
        return json({ error: 'Failed to download and save image' }, { status: 500 });
    }
};
