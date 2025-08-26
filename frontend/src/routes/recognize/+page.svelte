<script lang="ts">
	import { onMount } from 'svelte';
	import { getAppContext } from '$lib/context';
	import { validateImageFile, similarityToPercentage, type RecognitionResult } from '$lib/api';

	const { api, showMessage } = getAppContext();

	// Recognize state
	let recognizeFile: File | null = $state(null);
	let recognizeImageUrl: string | null = $state(null);
	let recognitionResults: RecognitionResult[] = $state([]);
	let recognizeLoading = $state(false);
	let imageLoaded = $state(false);
	let imageElement: HTMLImageElement | null = $state(null);

	// Face naming state
	let isNamingMode = $state(false);
	let selectedFaceIndex: number | null = $state(null);
	let selectedBbox: [number, number, number, number] | null = $state(null);
	let newFaceName = $state('');
	let namingLoading = $state(false);

	// Bounding box positions
	let bboxPositions: Array<{x1: number, y1: number, x2: number, y2: number, width: number, height: number}> = $state([]);

	// Track if data was loaded from Google Photos
	let loadedFromGooglePhotos = $state(false);

	// Handle file selection
	function handleRecognizeFileSelect(event: Event) {
		const target = event.target as HTMLInputElement;
		const file = target.files?.[0];

		if (!file) return;

		const validationError = validateImageFile(file);
		if (validationError) {
			showMessage(validationError, 'error');
			return;
		}

		recognizeFile = file;
		recognizeImageUrl = URL.createObjectURL(file);
		recognitionResults = [];
		imageLoaded = false;
		bboxPositions = [];
		isNamingMode = false;
		loadedFromGooglePhotos = false;
	}

	// Recognize faces
	async function recognizeFaces() {
		if (!recognizeFile) {
			showMessage('Please select an image first', 'error');
			return;
		}

		recognizeLoading = true;
		try {
			const results = await api.recognize(recognizeFile);
			recognitionResults = results;
			
			if (results.length === 0) {
				showMessage('No faces detected in the image', 'info');
			} else {
				showMessage(`Found ${results.length} face(s)`, 'success');
				calculateBboxPositions();
			}
		} catch (error: any) {
			console.error('Recognition failed:', error);
			showMessage(`Recognition failed: ${error.message || 'Unknown error'}`, 'error');
		} finally {
			recognizeLoading = false;
		}
	}

	// Calculate bbox positions
	function calculateBboxPositions() {
		if (!imageElement || !recognitionResults.length) {
			bboxPositions = [];
			return;
		}

		const imageNaturalWidth = imageElement.naturalWidth;
		const imageNaturalHeight = imageElement.naturalHeight;
		const imageDisplayWidth = imageElement.offsetWidth;
		const imageDisplayHeight = imageElement.offsetHeight;

		if (imageNaturalWidth === 0 || imageDisplayWidth === 0) {
			bboxPositions = [];
			return;
		}

		const scaleX = imageDisplayWidth / imageNaturalWidth;
		const scaleY = imageDisplayHeight / imageNaturalHeight;

		bboxPositions = recognitionResults.map((result) => {
			if (!result.bbox) {
				return { x1: 0, y1: 0, x2: 50, y2: 50, width: 50, height: 50 };
			}

			const x1 = result.bbox[0] * scaleX;
			const y1 = result.bbox[1] * scaleY;
			const x2 = result.bbox[2] * scaleX;
			const y2 = result.bbox[3] * scaleY;
			const width = x2 - x1;
			const height = y2 - y1;

			return { x1, y1, x2, y2, width, height };
		});
	}

	// Handle image load
	function handleImageLoad() {
		imageLoaded = true;
		calculateBboxPositions();
	}

	// Enable naming mode
	function enableNamingMode() {
		isNamingMode = true;
		showMessage('Click on an unknown face to name it', 'info');
	}

	// Handle face click for naming
	function handleFaceClick(index: number, bbox: [number, number, number, number]) {
		if (!isNamingMode) return;

		selectedFaceIndex = index;
		selectedBbox = bbox;
		newFaceName = '';
	}

	// Save face name
	async function saveFaceName() {
		if (!recognizeFile || selectedFaceIndex === null || !selectedBbox || !newFaceName.trim()) {
			showMessage('Missing required information for face naming', 'error');
			return;
		}

		namingLoading = true;
		try {
			await api.enrollFromBbox(newFaceName.trim(), recognizeFile, selectedBbox);
			showMessage(`Successfully enrolled ${newFaceName}!`, 'success');
			
			// Update the result
			if (selectedFaceIndex !== null) {
				recognitionResults[selectedFaceIndex].name = newFaceName.trim();
				recognitionResults[selectedFaceIndex].similarity = 1.0;
			}
			
			// Reset naming state
			selectedFaceIndex = null;
			selectedBbox = null;
			newFaceName = '';
			isNamingMode = false;
		} catch (error: any) {
			console.error('Face naming failed:', error);
			showMessage(`Face naming failed: ${error.message || 'Unknown error'}`, 'error');
		} finally {
			namingLoading = false;
		}
	}

	// Handle drag and drop
	function handleDrop(event: DragEvent) {
		event.preventDefault();
		const files = event.dataTransfer?.files;
		if (files && files.length > 0) {
			const file = files[0];
			const validationError = validateImageFile(file);
			if (validationError) {
				showMessage(validationError, 'error');
				return;
			}
			recognizeFile = file;
			recognizeImageUrl = URL.createObjectURL(file);
			recognitionResults = [];
			imageLoaded = false;
			bboxPositions = [];
			isNamingMode = false;
			loadedFromGooglePhotos = false;
		}
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
	}

	// Check for stored recognize data on mount
	onMount(() => {
		const storedData = sessionStorage.getItem('recognizeData');
		if (storedData) {
			try {
				const data = JSON.parse(storedData);

				// Check if data is recent (within 5 minutes)
				if (Date.now() - data.timestamp < 5 * 60 * 1000) {
					// Load the image and results
					recognizeImageUrl = data.imageUrl;
					recognitionResults = data.results;

					// Create a File object from the image URL for compatibility
					fetch(data.imageUrl)
						.then(response => response.blob())
						.then(blob => {
							recognizeFile = new File([blob], data.originalFilename, { type: blob.type });
						})
						.catch(error => {
							console.error('Failed to load image from URL:', error);
						});

					loadedFromGooglePhotos = true;
					showMessage(`Loaded analysis results for ${data.originalFilename}`, 'success');
				}

				// Clear the stored data after use
				sessionStorage.removeItem('recognizeData');
			} catch (error) {
				console.error('Failed to parse stored recognize data:', error);
				sessionStorage.removeItem('recognizeData');
			}
		}
	});

	// Reactive: recalculate positions when results change
	$effect(() => {
		if (recognitionResults.length > 0 && imageLoaded && imageElement) {
			calculateBboxPositions();
		}
	});
</script>

<div class="space-y-8">
	<div class="text-center">
		<h1 class="text-3xl font-bold text-green-400 mb-4 font-mono">// FACE RECOGNITION</h1>
		<p class="text-green-300 font-mono opacity-80">
			> Upload an image to identify faces using the trained recognition model
		</p>
		{#if loadedFromGooglePhotos}
			<div class="mt-2 inline-flex items-center space-x-2 px-3 py-1 bg-blue-900/30 border border-blue-400/30 rounded-lg">
				<span class="text-blue-400 text-sm">📷</span>
				<span class="text-blue-300 text-xs font-mono">Loaded from Google Photos</span>
			</div>
		{/if}
	</div>

	<div class="cyber-card rounded-lg p-8 max-w-4xl mx-auto">
		<!-- File Upload Area -->
		<div class="mb-6">
			<label for="recognize-file" class="block text-sm font-mono text-green-400 mb-2">
				> SELECT_IMAGE
			</label>
			<div
				class="border-2 border-dashed border-green-400/30 rounded-lg p-8 text-center hover:border-green-400/50 transition-colors cursor-pointer"
				role="button"
				tabindex="0"
				ondrop={handleDrop}
				ondragover={handleDragOver}
			>
				<input
					type="file"
					id="recognize-file"
					accept="image/*"
					onchange={handleRecognizeFileSelect}
					class="hidden"
				/>
				<label for="recognize-file" class="cursor-pointer">
					{#if recognizeImageUrl}
						<div class="relative inline-block">
							<img
								bind:this={imageElement}
								src={recognizeImageUrl}
								alt=""
								class="max-w-full max-h-96 mx-auto rounded-lg shadow-lg"
								onload={handleImageLoad}
							/>
							
							<!-- Face Overlays -->
							{#if imageLoaded && recognitionResults.length > 0 && bboxPositions.length > 0}
								{#each recognitionResults as result, index}
									{#if result.bbox && bboxPositions[index]}
										{@const pos = bboxPositions[index]}
										{@const percentage = similarityToPercentage(result.similarity)}
										
										<!-- Bounding Box -->
										{#if isNamingMode && result.name === 'Unknown'}
											<button
												class="absolute border-2 transition-all duration-300 border-gray-400 shadow-lg shadow-gray-400/20 cursor-pointer hover:border-cyan-400 hover:shadow-cyan-400/40 hover:bg-cyan-400/10 bg-transparent"
												style="left: {pos.x1}px; top: {pos.y1}px; width: {pos.width}px; height: {pos.height}px;"
												onclick={() => result.bbox ? handleFaceClick(index, result.bbox) : null}
												aria-label="Click to name this face"
												title="Click to name this face"
											></button>
										{:else}
											<div
												class="absolute border-2 transition-all duration-300 {result.name === 'Unknown' ? 'border-gray-400 shadow-lg shadow-gray-400/20' : percentage >= 75 ? 'border-green-400 shadow-lg shadow-green-400/30 cyber-glow' : percentage >= 50 ? 'border-yellow-400 shadow-lg shadow-yellow-400/30' : 'border-red-400 shadow-lg shadow-red-400/30'}"
												style="left: {pos.x1}px; top: {pos.y1}px; width: {pos.width}px; height: {pos.height}px;"
											></div>
										{/if}

										<!-- Label -->
										<div
											class="absolute px-2 py-1 text-xs font-mono font-bold text-black rounded-md backdrop-blur-sm border transition-all duration-300 {result.name === 'Unknown' ? 'bg-gray-400/90 border-gray-400' : percentage >= 75 ? 'bg-green-400/90 border-green-400 cyber-glow' : percentage >= 50 ? 'bg-yellow-400/90 border-yellow-400' : 'bg-red-400/90 border-red-400'}"
											style="left: {pos.x1}px; top: {Math.max(0, pos.y1 - 24)}px;"
										>
											<div class="whitespace-nowrap">
												{result.name.toUpperCase()}
												{#if result.name !== 'Unknown'}
													<span class="ml-1 opacity-80">({percentage.toFixed(0)}%)</span>
												{/if}
											</div>
										</div>
									{/if}
								{/each}
							{/if}
						</div>
						<p class="mt-4 text-sm text-green-300 font-mono">
							Click to change image or drag & drop a new one
						</p>
					{:else}
						<div class="text-green-400 text-4xl mb-4">🔍</div>
						<p class="text-green-300 font-mono">
							Click to select an image or drag & drop here
						</p>
						<p class="text-xs text-green-300/60 font-mono mt-2">
							Supports: JPG, PNG, WebP (max 15MB)
						</p>
					{/if}
				</label>
			</div>
		</div>

		<!-- Action Buttons -->
		<div class="flex flex-wrap gap-4 mb-6">
			<button
				onclick={recognizeFaces}
				disabled={!recognizeFile || recognizeLoading}
				class="cyber-button px-6 py-3 rounded-lg font-mono text-green-400 border border-green-400/30 hover:border-green-400/50 disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-300"
			>
				{#if recognizeLoading}
					<div class="flex items-center space-x-2">
						<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-green-400"></div>
						<span>> ANALYZING...</span>
					</div>
				{:else}
					> RECOGNIZE_FACES
				{/if}
			</button>

			{#if recognitionResults.some(r => r.name === 'Unknown')}
				<button
					onclick={enableNamingMode}
					disabled={isNamingMode}
					class="cyber-button px-6 py-3 rounded-lg font-mono text-cyan-400 border border-cyan-400/30 hover:border-cyan-400/50 disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-300"
				>
					> NAME_UNKNOWNS
				</button>
			{/if}
		</div>

		<!-- Results Summary -->
		{#if recognitionResults.length > 0}
			<div class="bg-green-900/20 border border-green-400/30 rounded-lg p-4 mb-6">
				<h3 class="text-sm font-mono text-green-400 mb-2">> RECOGNITION_RESULTS</h3>
				<div class="space-y-1">
					{#each recognitionResults as result, index}
						<div class="text-xs font-mono text-green-300">
							Face {index + 1}: {result.name} 
							{#if result.name !== 'Unknown'}
								<span class="text-green-400">({similarityToPercentage(result.similarity).toFixed(0)}% confidence)</span>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/if}
	</div>
</div>

<!-- Face Naming Modal -->
{#if selectedFaceIndex !== null}
	<div class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
		<div class="cyber-card rounded-lg p-8 max-w-md w-full mx-4">
			<h3 class="text-lg font-mono text-green-400 mb-4">> NAME_THIS_FACE</h3>
			
			<div class="mb-4">
				<label for="face-name" class="block text-sm font-mono text-green-400 mb-2">
					> PERSON_NAME
				</label>
				<input
					type="text"
					id="face-name"
					bind:value={newFaceName}
					placeholder="Enter person's name..."
					class="w-full px-4 py-3 bg-gray-800 border border-green-400/30 rounded-lg text-green-100 placeholder-green-300/50 focus:border-green-400 focus:outline-none font-mono"
					disabled={namingLoading}
				/>
			</div>

			<div class="flex space-x-4">
				<button
					onclick={saveFaceName}
					disabled={!newFaceName.trim() || namingLoading}
					class="flex-1 cyber-button px-4 py-2 rounded-lg font-mono text-green-400 border border-green-400/30 hover:border-green-400/50 disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-300"
				>
					{#if namingLoading}
						<div class="flex items-center justify-center space-x-2">
							<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-green-400"></div>
							<span>> SAVING...</span>
						</div>
					{:else}
						> CONFIRM
					{/if}
				</button>
				
				<button
					onclick={() => {
						selectedFaceIndex = null;
						selectedBbox = null;
						newFaceName = '';
					}}
					disabled={namingLoading}
					class="flex-1 cyber-button px-4 py-2 rounded-lg font-mono text-red-400 border border-red-400/30 hover:border-red-400/50 disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-300"
				>
					> CANCEL
				</button>
			</div>
		</div>
	</div>
{/if}
