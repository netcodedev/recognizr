<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { getAppContext } from '$lib/context';
	import { GooglePhotosAPI, type PickedMediaItem, type PickingSession, type SavedImage } from '$lib/api';

	const { api, showMessage } = getAppContext();

	// Google Photos state
	let googlePhotos = $state<GooglePhotosAPI>();
	let pickedPhotos: PickedMediaItem[] = $state([]);
	let savedImages: SavedImage[] = $state([]);
	let googlePhotosLoading = $state(false);
	let googlePhotoProcessing = $state(false);
	let currentSession: PickingSession | null = $state(null);
	let isPolling = $state(false);
	let savedImagesLoaded = $state(false);

	onMount(() => {
		// Initialize Google Photos Picker API in browser
		googlePhotos = new GooglePhotosAPI();

		const urlParams = new URLSearchParams(window.location.search);
		if (urlParams.get('google_photos_auth') === 'success') {
			showMessage('Google Photos authentication successful!', 'success');
			// Clean up URL
			window.history.replaceState({}, '', window.location.pathname);
		}

		// Load saved images
		loadSavedImages();
	});

	async function authenticateGooglePhotos() {
		if (!googlePhotos) return;
		
		try {
			const authUrl = googlePhotos.getAuthUrl();
			showMessage('Redirecting to Google for authentication...', 'info');
			window.location.href = authUrl;
		} catch (error: any) {
			console.error('Failed to start Google Photos authentication:', error);
			showMessage(`Failed to start authentication: ${error.message || 'Unknown error'}`, 'error');
		}
	}

	async function startPhotoPicking() {
		if (!googlePhotos || !googlePhotos.isAuthenticated()) {
			showMessage('Please authenticate with Google Photos first', 'error');
			return;
		}

		if (!googlePhotos.hasPickerScope()) {
			showMessage('Missing required scope. Please re-authenticate.', 'error');
			return;
		}

		googlePhotosLoading = true;

		try {
			const session = await googlePhotos.createSession();
			currentSession = session;
			
			showMessage('Opening Google Photos picker...', 'info');
			window.open(session.pickerUri, '_blank');
			
			isPolling = true;
			await googlePhotos.startPolling(
				session.id,
				async (completedSession) => {
					isPolling = false;
					showMessage('Photos selected! Loading your picks...', 'success');
					await loadPickedPhotos(completedSession.id);
				},
				(error) => {
					console.error('Polling error:', error);
					isPolling = false;

					if (error.message && error.message.includes('authentication')) {
						showMessage('Google Photos authentication expired. Please reconnect and try again.', 'error');
					} else {
						showMessage('Failed to get selected photos. Please try again.', 'error');
					}

					googlePhotosLoading = false;
				}
			);
			
		} catch (error: any) {
			console.error('Failed to start photo picking:', error);
			showMessage(`Failed to start photo picking: ${error.message || 'Unknown error'}`, 'error');
			googlePhotosLoading = false;
			isPolling = false;
		}
	}

	async function loadPickedPhotos(sessionId: string) {
		if (!googlePhotos) return;

		try {
			const response = await googlePhotos.listPickedMediaItems(sessionId);
			pickedPhotos = response.pickedMediaItems || response.mediaItems || [];

			if (pickedPhotos.length === 0) {
				showMessage('No photos were selected', 'info');
			} else {
				showMessage(`Importing ${pickedPhotos.length} selected photos...`, 'info');
				await importAllPickedPhotos();
			}

			await googlePhotos.deleteSession(sessionId);
			currentSession = null;
			
		} catch (error: any) {
			console.error('Failed to load picked photos:', error);
			showMessage(`Failed to load selected photos: ${error.message || 'Unknown error'}`, 'error');
		} finally {
			googlePhotosLoading = false;
		}
	}

	async function importAllPickedPhotos() {
		if (!googlePhotos || pickedPhotos.length === 0) return;

		try {
			const accessToken = googlePhotos.getToken()?.access_token;
			if (!accessToken) {
				throw new Error('No access token available');
			}

			const savedImages = await api.downloadMultipleGooglePhotos(pickedPhotos, accessToken);
			
			if (savedImages.length === pickedPhotos.length) {
				showMessage(`Successfully imported all ${savedImages.length} photos!`, 'success');
			} else {
				showMessage(`Imported ${savedImages.length} of ${pickedPhotos.length} photos (some failed)`, 'info');
			}

			pickedPhotos = [];
			savedImagesLoaded = false;
			await loadSavedImages();

		} catch (error: any) {
			console.error('Failed to import photos:', error);
			showMessage(`Failed to import photos: ${error.message || 'Unknown error'}`, 'error');
		}
	}

	async function analyzeSavedImage(savedImage: SavedImage) {
		googlePhotoProcessing = true;

		try {
			const results = await api.analyzeSavedImage(savedImage.id);

			if (results.length === 0) {
				showMessage(`No faces found in ${savedImage.original_filename}`, 'info');
			} else {
				showMessage(`Found ${results.length} face(s) in ${savedImage.original_filename}`, 'success');

				// Store the results and image data in sessionStorage for the recognize page
				const recognizeData = {
					imageUrl: `/images/${savedImage.filename}`,
					originalFilename: savedImage.original_filename,
					results: results,
					timestamp: Date.now()
				};

				sessionStorage.setItem('recognizeData', JSON.stringify(recognizeData));

				// Navigate to recognize page
				await goto('/recognize');
			}

		} catch (error: any) {
			console.error('Failed to analyze image:', error);
			showMessage(`Failed to analyze ${savedImage.original_filename}: ${error.message || 'Unknown error'}`, 'error');
		} finally {
			googlePhotoProcessing = false;
		}
	}

	async function loadSavedImages() {
		if (savedImagesLoaded) return;

		try {
			savedImages = await api.getSavedImages();
			savedImagesLoaded = true;
		} catch (error: any) {
			console.error('Failed to load saved images:', error);
			showMessage(`Failed to load saved images: ${error.message || 'Unknown error'}`, 'error');
		}
	}

	function disconnectGooglePhotos() {
		if (!googlePhotos) return;
		
		googlePhotos.clearAuth();
		pickedPhotos = [];
		currentSession = null;
		isPolling = false;
		showMessage('Disconnected from Google Photos', 'info');
	}

	function cancelPolling() {
		if (googlePhotos && isPolling) {
			googlePhotos.stopPolling();
			isPolling = false;
			googlePhotosLoading = false;
			showMessage('Photo selection cancelled', 'info');
		}
	}

	function forceReauthenticate() {
		if (!googlePhotos) return;
		
		googlePhotos.clearAuth();
		pickedPhotos = [];
		currentSession = null;
		isPolling = false;
		
		showMessage('Please authenticate again...', 'info');
		setTimeout(() => {
			authenticateGooglePhotos();
		}, 500);
	}
</script>

<div class="space-y-8">
	<div class="text-center">
		<h1 class="text-3xl font-bold text-green-400 mb-4 font-mono">// GOOGLE PHOTOS LIBRARY</h1>
		<p class="text-green-300 font-mono opacity-80">
			> Import photos from Google Photos and analyze faces in your saved library
		</p>
	</div>

	<div class="cyber-card rounded-lg p-8">
		<!-- Info Box -->
		<div class="bg-green-900/20 border border-green-400/30 rounded-lg p-4 mb-8">
			<p class="text-xs text-green-300 font-mono mb-2">
				<span class="text-green-400">INFO:</span> Import photos from Google Photos to your local library, then analyze faces.
				Photos are downloaded and saved locally for faster access and offline analysis.
			</p>
			<p class="text-xs text-yellow-300 font-mono">
				<span class="text-yellow-400">PRIVACY:</span> Only photos you explicitly select will be imported to your local library.
			</p>
		</div>

		{#if !googlePhotos?.isAuthenticated()}
			<!-- Not authenticated -->
			<div class="text-center">
				<button
					onclick={authenticateGooglePhotos}
					class="cyber-button px-8 py-4 rounded-lg text-lg font-mono text-green-400 border-green-400/50 hover:border-green-400/80 transition-all duration-300"
				>
					> CONNECT_GOOGLE_PHOTOS
				</button>
			</div>
		{:else}
			<!-- Authenticated -->
			<div class="space-y-6">
				<!-- Header with controls -->
				<div class="flex justify-between items-center">
					<div class="text-green-300 font-mono text-sm">
						> Connected to Google Photos
						{#if googlePhotos.hasPickerScope()}
							<span class="text-green-400 ml-2">✓ Picker API Ready</span>
						{:else}
							<span class="text-yellow-400 ml-2">⚠ Missing Picker Scope</span>
						{/if}
					</div>
					<div class="flex gap-2">
						{#if isPolling}
							<button
								onclick={cancelPolling}
								class="cyber-button px-4 py-2 rounded-md text-xs font-mono text-yellow-400 border-yellow-400/30 hover:border-yellow-400/50"
							>
								> CANCEL_SELECTION
							</button>
						{/if}
						<button
							onclick={forceReauthenticate}
							class="cyber-button px-4 py-2 rounded-md text-xs font-mono text-yellow-400 border-yellow-400/30 hover:border-yellow-400/50"
						>
							> REAUTH
						</button>
						<button
							onclick={disconnectGooglePhotos}
							class="cyber-button px-4 py-2 rounded-md text-xs font-mono text-red-400 border-red-400/30 hover:border-red-400/50"
						>
							> DISCONNECT
						</button>
					</div>
				</div>

				<!-- Photo Selection -->
				{#if !isPolling && pickedPhotos.length === 0}
					<div class="text-center">
						<button
							onclick={startPhotoPicking}
							disabled={googlePhotosLoading}
							class="cyber-button px-8 py-4 rounded-lg text-lg font-mono text-green-400 border-green-400/50 hover:border-green-400/80 disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-300"
						>
							{#if googlePhotosLoading}
								> CREATING_SESSION...
							{:else}
								> IMPORT_FROM_GOOGLE_PHOTOS
							{/if}
						</button>
						<p class="text-xs text-green-300/60 font-mono mt-4">
							Click to open Google Photos and select photos to import to your library
						</p>
					</div>
				{/if}

				<!-- Polling status -->
				{#if isPolling}
					<div class="text-center">
						<div class="inline-flex items-center space-x-2 text-green-400 font-mono">
							<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-green-400"></div>
							<span>Waiting for photo selection...</span>
						</div>
						<p class="text-xs text-green-300/60 font-mono mt-2">
							Select photos in the Google Photos window, then click "Done" to import them
						</p>
					</div>
				{/if}

				<!-- Import status message -->
				{#if pickedPhotos.length > 0}
					<div class="text-center py-8">
						<div class="inline-flex items-center space-x-2 text-green-400 font-mono">
							<div class="animate-spin rounded-full h-6 w-6 border-b-2 border-green-400"></div>
							<span>Importing {pickedPhotos.length} selected photos...</span>
						</div>
						<p class="text-xs text-green-300/60 font-mono mt-2">
							Photos will appear in the library below once imported
						</p>
					</div>
				{/if}

				<!-- Saved Images Library -->
				<div class="mt-8">
					<div class="flex justify-between items-center mb-4">
						<h3 class="text-lg font-mono text-green-400">
							> Saved Images Library ({savedImages.length})
						</h3>
						<button
							onclick={() => { savedImagesLoaded = false; loadSavedImages(); }}
							class="cyber-button px-4 py-2 rounded-md text-xs font-mono text-green-400 border-green-400/30 hover:border-green-400/50"
						>
							> REFRESH
						</button>
					</div>

					{#if savedImages.length === 0}
						<div class="text-center py-8">
							<div class="text-green-400/50 text-4xl mb-4">📷</div>
							<p class="text-green-300 font-mono text-sm">No images in library yet</p>
							<p class="text-green-300/60 font-mono text-xs mt-2">Import photos from Google Photos to get started</p>
						</div>
					{:else}
						<div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
							{#each savedImages as savedImage}
								<div class="relative group">
									<div class="aspect-square bg-gray-800 rounded-lg overflow-hidden border border-green-400/20 hover:border-green-400/50 transition-all duration-300">
										<img
											src="/images/{savedImage.filename}"
											alt={savedImage.original_filename}
											class="w-full h-full object-cover"
											loading="lazy"
										/>
										
										<!-- Overlay with actions -->
										<div class="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity duration-300 flex items-center justify-center">
											<button
												onclick={() => analyzeSavedImage(savedImage)}
												disabled={googlePhotoProcessing}
												class="cyber-button px-3 py-1 rounded text-xs font-mono text-green-400 border-green-400/50 hover:border-green-400/80 disabled:opacity-50"
											>
												{#if googlePhotoProcessing}
													> ANALYZING...
												{:else}
													> ANALYZE_FACES
												{/if}
											</button>
										</div>
									</div>
									
									<!-- Photo info -->
									<div class="mt-2 text-xs text-green-300/80 font-mono truncate">
										{savedImage.original_filename}
									</div>
									<div class="text-xs text-green-300/60 font-mono">
										{new Date(savedImage.created_at).toLocaleDateString()}
									</div>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</div>
