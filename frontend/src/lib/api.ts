// API configuration and types for the Recognizr backend

export const API_BASE = 'http://localhost:3000';

// Type definitions matching the Rust backend
export interface RecognitionResult {
	name: string;
	similarity: number;
	bbox?: [number, number, number, number]; // [x1, y1, x2, y2]
}

export interface ApiError {
	message: string;
	status: number;
}

// API utility functions
export class RecognizrAPI {
	private baseUrl: string;

	constructor(baseUrl: string = API_BASE) {
		this.baseUrl = baseUrl;
	}

	/**
	 * Enroll a person with their face image
	 */
	async enroll(name: string, imageFile: File): Promise<void> {
		const formData = new FormData();
		formData.append('name', name.trim());
		formData.append('image', imageFile);

		const response = await fetch(`${this.baseUrl}/enroll`, {
			method: 'POST',
			body: formData
		});

		if (!response.ok) {
			const errorText = await response.text();
			throw new ApiError(errorText, response.status);
		}
	}

	/**
	 * Recognize faces in an image
	 */
	async recognize(imageFile: File): Promise<RecognitionResult[]> {
		const formData = new FormData();
		formData.append('image', imageFile);

		const response = await fetch(`${this.baseUrl}/recognize`, {
			method: 'POST',
			body: formData
		});

		if (!response.ok) {
			const errorText = await response.text();
			throw new ApiError(errorText, response.status);
		}

		return await response.json();
	}

	/**
	 * Generate debug image with face detection annotations
	 */
	async debug(imageFile: File, threshold: number = 0.6): Promise<Blob> {
		const formData = new FormData();
		formData.append('image', imageFile);

		const url = `${this.baseUrl}/debug/detector?threshold=${threshold}`;
		const response = await fetch(url, {
			method: 'POST',
			body: formData
		});

		if (!response.ok) {
			const errorText = await response.text();
			throw new ApiError(errorText, response.status);
		}

		return await response.blob();
	}

	/**
	 * Check if the API server is reachable
	 */
	async healthCheck(): Promise<{status: string, service: string, version: string} | null> {
		try {
			const response = await fetch(`${this.baseUrl}/health`, {
				method: 'GET',
				signal: AbortSignal.timeout(5000) // 5 second timeout
			});
			if (response.ok) {
				return await response.json();
			}
			return null;
		} catch {
			return null;
		}
	}
}

// Custom error class for API errors
export class ApiError extends Error {
	public status: number;

	constructor(message: string, status: number) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
	}
}

// Utility functions for file validation
export function validateImageFile(file: File): string | null {
	// Check file type
	if (!file.type.startsWith('image/')) {
		return 'Please select a valid image file';
	}

	// Check file size (15MB limit to match backend)
	const maxSize = 15 * 1024 * 1024; // 15MB
	if (file.size > maxSize) {
		return 'Image too large (max 15MB)';
	}

	// Check for empty file
	if (file.size === 0) {
		return 'Image file is empty';
	}

	return null; // No validation errors
}

// Format similarity score as percentage
export function formatSimilarity(similarity: number): string {
	return `${(similarity * 100).toFixed(1)}%`;
}

// Get confidence level based on similarity score
export function getConfidenceLevel(similarity: number): 'high' | 'medium' | 'low' {
	if (similarity >= 0.7) return 'high';
	if (similarity >= 0.45) return 'medium';
	return 'low';
}

// Get confidence level styling classes
export function getConfidenceClasses(similarity: number): string {
	const level = getConfidenceLevel(similarity);
	switch (level) {
		case 'high':
			return 'bg-green-100 text-green-800';
		case 'medium':
			return 'bg-yellow-100 text-yellow-800';
		case 'low':
			return 'bg-red-100 text-red-800';
	}
}
