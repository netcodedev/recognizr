// API configuration and types for the Recognizr backend
import { PUBLIC_GOOGLE_CLIENT_ID, PUBLIC_GOOGLE_CLIENT_SECRET } from '$env/static/public';

export const API_BASE = 'http://localhost:3000';

// Google Photos API configuration
export const GOOGLE_PHOTOS_CONFIG = {
    clientId: PUBLIC_GOOGLE_CLIENT_ID,
    redirectUri: 'http://localhost:5173/auth/callback',
    scope: 'https://www.googleapis.com/auth/photospicker.mediaitems.readonly https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile',
    authUrl: 'https://accounts.google.com/o/oauth2/auth',
    tokenUrl: 'https://oauth2.googleapis.com/token',
    pickerApiBaseUrl: 'https://photospicker.googleapis.com/v1',
    client_secret: PUBLIC_GOOGLE_CLIENT_SECRET
};

// Google Photos Picker API types
export interface PickingSession {
    id: string;
    pickerUri: string;
    mediaItemsSet: boolean;
    pollingConfig?: {
        pollIntervalMillis: number;
        timeoutMillis: number;
    };
}

export interface PickedMediaItem {
    id: string;
    filename?: string;
    createTime?: string; // The actual API uses this field
    type?: string;
    mediaFile: {
        baseUrl: string;
        mimeType?: string;
    };
    mediaMetadata?: {
        creationTime?: string;
        width?: string;
        height?: string;
    };
}

export interface PickedMediaItemsResponse {
    pickedMediaItems?: PickedMediaItem[];
    mediaItems?: PickedMediaItem[]; // The actual API returns this field
    nextPageToken?: string;
}

// Type definitions matching the Rust backend
export interface RecognitionResult {
	name: string;
	similarity: number; // Range: -1.0 to 1.0 (cosine similarity)
	bbox?: [number, number, number, number]; // [x1, y1, x2, y2] in image coordinates
}

export interface GalleryPerson {
	name: string;
	image_base64: string; // Base64 encoded JPEG image
}

export interface ApiError {
	message: string;
	status: number;
}

// Saved image types
export interface SavedImage {
    id: string;
    filename: string;
    original_filename: string;
    file_path: string;
    created_at: string;
    source: string;
}

// Google Photos API types
export interface GooglePhotosToken {
	access_token: string;
	refresh_token?: string;
	expires_in: number;
	token_type: string;
	scope: string;
	expires_at?: number; // Calculated expiration timestamp
}

export interface MediaItem {
	id: string;
	description?: string;
	productUrl: string;
	baseUrl: string;
	mimeType: string;
	mediaMetadata: {
		creationTime: string;
		width: string;
		height: string;
		photo?: {
			cameraMake?: string;
			cameraModel?: string;
			focalLength?: number;
			apertureFNumber?: number;
			isoEquivalent?: number;
			exposureTime?: string;
		};
	};
	contributorInfo?: {
		profilePictureBaseUrl: string;
		displayName: string;
	};
	filename: string;
}

export interface MediaItemsResponse {
	mediaItems?: MediaItem[];
	nextPageToken?: string;
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
	 * Enroll a person from a specific face in an image using bounding box coordinates
	 */
	async enrollFromBbox(name: string, imageFile: File, bbox: [number, number, number, number]): Promise<void> {
		const formData = new FormData();
		formData.append('name', name.trim());
		formData.append('image', imageFile);
		formData.append('bbox', bbox.join(','));

		const response = await fetch(`${this.baseUrl}/enroll-from-bbox`, {
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
	 * Get gallery of all enrolled people with their cropped images
	 */
	async getGallery(): Promise<GalleryPerson[]> {
		const response = await fetch(`${this.baseUrl}/gallery`, {
			method: 'GET'
		});

		if (!response.ok) {
			const errorText = await response.text();
			throw new ApiError(errorText, response.status);
		}

		return await response.json();
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

		/**
		 * Get all saved images
		 */
		async getSavedImages(): Promise<SavedImage[]> {
			const response = await fetch('/api/images', {
				method: 'GET'
			});

			if (!response.ok) {
				const errorText = await response.text();
				throw new ApiError(errorText, response.status);
			}

			return await response.json();
		}

		/**
		 * Download and save a Google Photos image
		 */
		async downloadGooglePhoto(mediaItem: PickedMediaItem, accessToken: string): Promise<SavedImage> {
			const response = await fetch('/api/images', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({
					media_item: mediaItem,
					access_token: accessToken
				})
			});

			if (!response.ok) {
				const errorText = await response.text();
				throw new ApiError(errorText, response.status);
			}

			return await response.json();
		}

		/**
		 * Analyze faces in a saved image
		 */
		async analyzeSavedImage(imageId: string): Promise<RecognitionResult[]> {
			const response = await fetch(`/api/images/${imageId}/analyze`, {
				method: 'POST'
			});

			if (!response.ok) {
				const errorText = await response.text();
				throw new ApiError(errorText, response.status);
			}

			return await response.json();
		}

		/**
		 * Download and save multiple Google Photos images
		 */
		async downloadMultipleGooglePhotos(mediaItems: PickedMediaItem[], accessToken: string): Promise<SavedImage[]> {
			const savedImages: SavedImage[] = [];

			for (const mediaItem of mediaItems) {
				try {
					const savedImage = await this.downloadGooglePhoto(mediaItem, accessToken);
					savedImages.push(savedImage);
				} catch (error) {
					console.error(`Failed to download ${mediaItem.filename || mediaItem.id}:`, error);
					// Continue with other images even if one fails
				}
			}

			return savedImages;
		}
}

// Google Photos Picker API client
export class GooglePhotosAPI {
    private token: GooglePhotosToken | null = null;
    private currentSession: PickingSession | null = null;
    private pollingInterval: number | null = null;

    constructor() {
        // Try to load token from localStorage
        this.loadTokenFromStorage();
    }

    /**
     * Generate OAuth authorization URL
     */
    getAuthUrl(state: string = ''): string {
        // Use current origin for redirect URI to handle different ports
        const redirectUri = typeof window !== 'undefined' 
            ? `${window.location.origin}/auth/callback`
            : GOOGLE_PHOTOS_CONFIG.redirectUri;

        const params = new URLSearchParams({
            client_id: GOOGLE_PHOTOS_CONFIG.clientId,
            redirect_uri: redirectUri,
            scope: GOOGLE_PHOTOS_CONFIG.scope,
            response_type: 'code',
            access_type: 'offline',
            prompt: 'consent',
            state: state
        });

        return `${GOOGLE_PHOTOS_CONFIG.authUrl}?${params.toString()}`;
    }

    /**
     * Exchange authorization code for access token
     */
    async exchangeCodeForToken(code: string): Promise<GooglePhotosToken> {
        const redirectUri = typeof window !== 'undefined' 
            ? `${window.location.origin}/auth/callback`
            : GOOGLE_PHOTOS_CONFIG.redirectUri;

        const response = await fetch(GOOGLE_PHOTOS_CONFIG.tokenUrl, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/x-www-form-urlencoded',
            },
            body: new URLSearchParams({
                client_id: GOOGLE_PHOTOS_CONFIG.clientId,
                client_secret: GOOGLE_PHOTOS_CONFIG.client_secret,
                code: code,
                grant_type: 'authorization_code',
                redirect_uri: redirectUri
            })
        });

        if (!response.ok) {
            let errorMessage = `HTTP ${response.status}`;
            try {
                const errorText = await response.text();
                try {
                    const errorJson = JSON.parse(errorText);
                    errorMessage = errorJson.error_description || errorJson.error || errorText;
                } catch {
                    errorMessage = errorText || errorMessage;
                }
            } catch {
                // If we can't read the response, use the status
            }
            throw new ApiError(`Token exchange failed: ${errorMessage}`, response.status);
        }

        const tokenData = await response.json();
        
        const token: GooglePhotosToken = {
            access_token: tokenData.access_token,
            refresh_token: tokenData.refresh_token,
            expires_in: tokenData.expires_in || 3600,
            token_type: tokenData.token_type || 'Bearer',
            scope: tokenData.scope || GOOGLE_PHOTOS_CONFIG.scope,
            expires_at: Date.now() + (tokenData.expires_in || 3600) * 1000
        };

        this.setToken(token);
        return token;
    }

    /**
     * Set the access token
     */
    setToken(token: GooglePhotosToken): void {
        this.token = token;
        this.saveTokenToStorage();
    }

    /**
     * Get the current token
     */
    getToken(): GooglePhotosToken | null {
        return this.token;
    }

    /**
     * Check if user is authenticated
     */
    isAuthenticated(): boolean {
        const hasToken = this.token !== null;
        const isExpired = this.isTokenExpired();
        const result = hasToken && !isExpired;

        return result;
    }

    /**
     * Check if token is expired
     */
    private isTokenExpired(): boolean {
        if (!this.token || !this.token.expires_at) return true;
        return Date.now() >= this.token.expires_at;
    }

    /**
     * Get the scopes that were granted
     */
    getGrantedScopes(): string[] {
        if (!this.token) return [];
        return this.token.scope.split(' ');
    }

    /**
     * Check if we have the Picker API scope
     */
    hasPickerScope(): boolean {
        return this.getGrantedScopes().includes('https://www.googleapis.com/auth/photospicker.mediaitems.readonly');
    }

    /**
     * Clear authentication
     */
    clearAuth(): void {
        this.token = null;
        this.currentSession = null;
        this.stopPolling();
        if (typeof window !== 'undefined') {
            localStorage.removeItem('google_photos_token');
        }
    }

    /**
     * Save token to localStorage
     */
    private saveTokenToStorage(): void {
        if (typeof window !== 'undefined' && this.token) {
            localStorage.setItem('google_photos_token', JSON.stringify(this.token));
        }
    }

    /**
     * Load token from localStorage
     */
    private loadTokenFromStorage(): void {
        if (typeof window !== 'undefined') {
            const stored = localStorage.getItem('google_photos_token');
            if (stored) {
                try {
                    this.token = JSON.parse(stored);
                } catch {
                    // Invalid token, clear it
                    localStorage.removeItem('google_photos_token');
                }
            }
        }
    }

    /**
     * Create a new picking session
     */
    async createSession(): Promise<PickingSession> {
        if (!this.isAuthenticated()) {
            throw new ApiError('Not authenticated with Google Photos', 401);
        }

        const response = await fetch(`${GOOGLE_PHOTOS_CONFIG.pickerApiBaseUrl}/sessions`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${this.token!.access_token}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({})
        });

        if (!response.ok) {
            if (response.status === 401) {
                this.clearAuth();
                throw new ApiError('Google Photos authentication expired. Please reconnect.', 401);
            }
            
            let errorMessage = `HTTP ${response.status}`;
            try {
                const errorText = await response.text();
                try {
                    const errorJson = JSON.parse(errorText);
                    errorMessage = errorJson.error?.message || errorJson.error_description || errorText;
                } catch {
                    errorMessage = errorText || errorMessage;
                }
            } catch {
                // If we can't read the response, use the status
            }
            
            throw new ApiError(`Failed to create session: ${errorMessage}`, response.status);
        }

        const session = await response.json();
        this.currentSession = session;
        return session;
    }

    /**
     * Get session status
     */
    async getSession(sessionId: string): Promise<PickingSession> {
        if (!this.isAuthenticated()) {
            console.error('Token authentication failed:', {
                hasToken: !!this.token,
                tokenExpired: this.token ? this.isTokenExpired() : 'no token',
                expiresAt: this.token?.expires_at,
                currentTime: Date.now()
            });
            throw new ApiError('Not authenticated with Google Photos', 401);
        }

        const response = await fetch(`${GOOGLE_PHOTOS_CONFIG.pickerApiBaseUrl}/sessions/${sessionId}`, {
            method: 'GET',
            headers: {
                'Authorization': `Bearer ${this.token!.access_token}`,
                'Content-Type': 'application/json'
            }
        });

        if (!response.ok) {
            if (response.status === 401) {
                console.error('Session request returned 401, clearing auth');
                this.clearAuth();
                throw new ApiError('Google Photos authentication expired. Please reconnect.', 401);
            }

            let errorMessage = `HTTP ${response.status}`;
            try {
                const errorText = await response.text();
                try {
                    const errorJson = JSON.parse(errorText);
                    errorMessage = errorJson.error?.message || errorJson.error_description || errorText;
                } catch {
                    errorMessage = errorText || errorMessage;
                }
            } catch {
                // If we can't read the response, use the status
            }

            throw new ApiError(`Failed to get session: ${errorMessage}`, response.status);
        }

        const session = await response.json();
        this.currentSession = session;
        return session;
    }

    /**
     * Start polling session until photos are selected
     */
    async startPolling(sessionId: string, onComplete: (session: PickingSession) => void, onError: (error: Error) => void): Promise<void> {

        const poll = async () => {
            try {
                const session = await this.getSession(sessionId);

                if (session.mediaItemsSet) {
                    this.stopPolling();
                    onComplete(session);
                    return;
                }

                // Use recommended polling interval or default to 2 seconds
                const pollInterval = session.pollingConfig?.pollIntervalMillis || 2000;
                this.pollingInterval = window.setTimeout(poll, pollInterval);
            } catch (error) {
                console.error('Polling error:', error);
                this.stopPolling();

                // If it's an authentication error, clear the auth state
                if (error instanceof ApiError && error.status === 401) {
                    this.clearAuth();
                }

                onError(error as Error);
            }
        };

        // Start polling
        poll();
    }

    /**
     * Stop polling
     */
    stopPolling(): void {
        if (this.pollingInterval) {
            clearTimeout(this.pollingInterval);
            this.pollingInterval = null;
        }
    }

    /**
     * List picked media items
     */
    async listPickedMediaItems(sessionId: string, pageToken?: string): Promise<PickedMediaItemsResponse> {
        if (!this.isAuthenticated()) {
            throw new ApiError('Not authenticated with Google Photos', 401);
        }

        const url = new URL(`${GOOGLE_PHOTOS_CONFIG.pickerApiBaseUrl}/mediaItems`);
        url.searchParams.set('sessionId', sessionId);
        if (pageToken) {
            url.searchParams.set('pageToken', pageToken);
        }

        const response = await fetch(url.toString(), {
            method: 'GET',
            headers: {
                'Authorization': `Bearer ${this.token!.access_token}`,
                'Content-Type': 'application/json'
            }
        });

        if (!response.ok) {
            const errorText = await response.text();
            console.error('listPickedMediaItems error response:', errorText);
            throw new ApiError(`Failed to list picked media items: ${response.status} - ${errorText}`, response.status);
        }

        const result = await response.json();
        return result;
    }

    /**
     * Download a picked photo
     */
    async downloadPickedPhoto(mediaItem: PickedMediaItem): Promise<Blob> {
        if (!this.isAuthenticated()) {
            throw new ApiError('Not authenticated with Google Photos', 401);
        }

        // Add download parameter to base URL
        const downloadUrl = `${mediaItem.mediaFile.baseUrl}=d`;

        const response = await fetch(downloadUrl, {
            headers: {
                'Authorization': `Bearer ${this.token!.access_token}`
            }
        });

        if (!response.ok) {
            const errorText = await response.text();
            console.error('Download error response:', errorText);
            const filename = mediaItem.filename || `photo_${mediaItem.id}`;
            throw new ApiError(`Failed to download photo: ${filename} (${response.status})`, response.status);
        }

        return await response.blob();
    }

    /**
     * Delete session (cleanup)
     */
    async deleteSession(sessionId: string): Promise<void> {
        if (!this.isAuthenticated()) {
            return;
        }

        try {
            await fetch(`${GOOGLE_PHOTOS_CONFIG.pickerApiBaseUrl}/sessions/${sessionId}`, {
                method: 'DELETE',
                headers: {
                    'Authorization': `Bearer ${this.token!.access_token}`
                }
            });
        } catch (error) {
            console.warn('Failed to delete session:', error);
        }
    }

    /**
     * Get current session
     */
    getCurrentSession(): PickingSession | null {
        return this.currentSession;
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

// Convert similarity score (-1 to 1) to percentage (0 to 100)
export function similarityToPercentage(similarity: number): number {
	return Math.max(0, Math.min(100, (similarity + 1) * 50));
}

// Format similarity score as percentage (adjusted for -1 to 1 range)
export function formatSimilarity(similarity: number): string {
	return `${similarityToPercentage(similarity).toFixed(1)}%`;
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
