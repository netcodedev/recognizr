<script lang="ts">
	import { RecognizrAPI, validateImageFile, similarityToPercentage, API_BASE, type RecognitionResult, type GalleryPerson } from '$lib/api';

	// API instance
	const api = new RecognizrAPI();

	// Component state
	let activeTab: 'enroll' | 'recognize' | 'gallery' | 'debug' = 'enroll';
	let isLoading = false;
	let message = '';
	let messageType: 'success' | 'error' | 'info' = 'info';

	// Enroll form state
	let enrollName = '';
	let enrollFile: File | null = null;
	let enrollFileInput: HTMLInputElement;

	// Recognize form state
	let recognizeFile: File | null = null;
	let recognizeFileInput: HTMLInputElement;
	let recognitionResults: RecognitionResult[] = [];
	let recognizeImageUrl = '';
	let imageElement: HTMLImageElement;
	let imageLoaded = false;

	// Debug form state
	let debugFile: File | null = null;
	let debugFileInput: HTMLInputElement;
	let debugThreshold = 0.6;
	let debugImageUrl = '';

	// API status
	let apiStatus: 'checking' | 'connected' | 'disconnected' = 'checking';
	let apiInfo: {status: string, service: string, version: string} | null = null;

	// Face naming state
	let isNamingMode = false;
	let selectedFaceIndex: number | null = null;
	let newFaceName = '';
	let showNameDialog = false;

	// Gallery state
	let galleryPeople: GalleryPerson[] = [];
	let galleryLoaded = false;

	function showMessage(text: string, type: 'success' | 'error' | 'info' = 'info') {
		message = text;
		messageType = type;
		setTimeout(() => {
			message = '';
		}, 5000);
	}

	async function checkApiStatus() {
		apiStatus = 'checking';
		try {
			const result = await api.healthCheck();
			if (result) {
				apiStatus = 'connected';
				apiInfo = result;
			} else {
				apiStatus = 'disconnected';
				apiInfo = null;
			}
		} catch {
			apiStatus = 'disconnected';
			apiInfo = null;
		}
	}

	// Check API status on component mount
	checkApiStatus();

	// Function to load gallery
	async function loadGallery() {
		if (galleryLoaded) return;

		isLoading = true;
		try {
			galleryPeople = await api.getGallery();
			galleryLoaded = true;
		} catch (error: any) {
			showMessage(`Failed to load gallery: ${error.message}`, 'error');
		} finally {
			isLoading = false;
		}
	}

	// Load gallery when gallery tab is selected
	$: if (activeTab === 'gallery') {
		loadGallery();
	}

	// Test function to simulate recognition results (for development/testing)
	function addTestResults() {
		if (!recognizeImageUrl || !imageElement) return;

		// Simulate some test results with different confidence levels
		recognitionResults = [
			{
				name: "John Doe",
				similarity: 0.85, // High confidence
				bbox: [100, 50, 200, 180] // [x1, y1, x2, y2]
			},
			{
				name: "Jane Smith",
				similarity: 0.45, // Medium confidence
				bbox: [300, 80, 400, 210]
			},
			{
				name: "Unknown",
				similarity: 0.15, // Low confidence
				bbox: [150, 250, 250, 380]
			}
		];
	}

	// Function to handle face click for naming
	function handleFaceClick(faceIndex: number, bbox: [number, number, number, number]) {
		if (!isNamingMode) return;

		selectedFaceIndex = faceIndex;
		newFaceName = '';
		showNameDialog = true;
	}

	// Function to save the new face name
	async function saveFaceName() {
		if (!newFaceName.trim() || selectedFaceIndex === null || !recognizeFile) {
			showMessage('Please enter a valid name', 'error');
			return;
		}

		const selectedResult = recognitionResults[selectedFaceIndex];
		if (!selectedResult.bbox) {
			showMessage('No bounding box data for selected face', 'error');
			return;
		}

		isLoading = true;

		try {
			await api.enrollFromBbox(newFaceName.trim(), recognizeFile, selectedResult.bbox);
			showMessage(`Successfully enrolled ${newFaceName}!`, 'success');

			// Update the recognition result to show the new name
			recognitionResults[selectedFaceIndex].name = newFaceName.trim();
			recognitionResults[selectedFaceIndex].similarity = 1.0; // High confidence for user-labeled face

			// Refresh gallery if it was loaded
			if (galleryLoaded) {
				galleryLoaded = false;
			}

			// Close dialog and reset state
			showNameDialog = false;
			selectedFaceIndex = null;
			newFaceName = '';
			isNamingMode = false;
		} catch (error: any) {
			showMessage(`Failed to enroll face: ${error.message}`, 'error');
		} finally {
			isLoading = false;
		}
	}

	function cancelNaming() {
		showNameDialog = false;
		selectedFaceIndex = null;
		newFaceName = '';
		isNamingMode = false;
	}

	async function handleEnroll() {
		if (!enrollName.trim()) {
			showMessage('Please enter a name', 'error');
			return;
		}

		if (!enrollFile) {
			showMessage('Please select an image', 'error');
			return;
		}

		// Validate file
		const validationError = validateImageFile(enrollFile);
		if (validationError) {
			showMessage(validationError, 'error');
			return;
		}

		isLoading = true;

		try {
			await api.enroll(enrollName.trim(), enrollFile);
			showMessage(`Successfully enrolled ${enrollName}!`, 'success');
			enrollName = '';
			enrollFile = null;
			enrollFileInput.value = '';

			// Refresh gallery if it was loaded
			if (galleryLoaded) {
				galleryLoaded = false;
			}
		} catch (error: any) {
			showMessage(`Enrollment failed: ${error.message}`, 'error');
		} finally {
			isLoading = false;
		}
	}

	async function handleRecognize() {
		if (!recognizeFile) {
			showMessage('Please select an image', 'error');
			return;
		}

		// Validate file
		const validationError = validateImageFile(recognizeFile);
		if (validationError) {
			showMessage(validationError, 'error');
			return;
		}

		isLoading = true;
		recognitionResults = [];
		imageLoaded = false;

		// Create image URL for display
		if (recognizeImageUrl) {
			URL.revokeObjectURL(recognizeImageUrl);
		}
		recognizeImageUrl = URL.createObjectURL(recognizeFile);

		try {
			const results = await api.recognize(recognizeFile);
			recognitionResults = results;
			showMessage(`Found ${results.length} face(s)`, 'success');
		} catch (error: any) {
			showMessage(`Recognition failed: ${error.message}`, 'error');
		} finally {
			isLoading = false;
		}
	}

	// Handle image load event to enable overlay positioning
	function handleImageLoad() {
		imageLoaded = true;
	}

	async function handleDebug() {
		if (!debugFile) {
			showMessage('Please select an image', 'error');
			return;
		}

		// Validate file
		const validationError = validateImageFile(debugFile);
		if (validationError) {
			showMessage(validationError, 'error');
			return;
		}

		isLoading = true;
		debugImageUrl = '';

		try {
			const blob = await api.debug(debugFile, debugThreshold);
			debugImageUrl = URL.createObjectURL(blob);
			showMessage('Debug image generated successfully!', 'success');
		} catch (error: any) {
			showMessage(`Debug failed: ${error.message}`, 'error');
		} finally {
			isLoading = false;
		}
	}

	function handleFileSelect(event: Event, type: 'enroll' | 'recognize' | 'debug') {
		const target = event.target as HTMLInputElement;
		const file = target.files?.[0];

		if (file) {
			// Validate file using utility function
			const validationError = validateImageFile(file);
			if (validationError) {
				showMessage(validationError, 'error');
				target.value = '';
				return;
			}

			switch (type) {
				case 'enroll':
					enrollFile = file;
					break;
				case 'recognize':
					recognizeFile = file;
					// Clear previous results and image
					recognitionResults = [];
					imageLoaded = false;
					if (recognizeImageUrl) {
						URL.revokeObjectURL(recognizeImageUrl);
						recognizeImageUrl = '';
					}
					break;
				case 'debug':
					debugFile = file;
					break;
			}
		}
	}
</script>

<svelte:head>
	<title>Recognizr - Face Recognition App</title>
	<meta name="description" content="Face recognition application using the Recognizr API" />
</svelte:head>

<div class="min-h-screen bg-gray-50">
	<!-- Header -->
	<header class="bg-white shadow-sm border-b">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex justify-between items-center py-6">
				<div class="flex items-center">
					<h1 class="text-3xl font-bold text-gray-900">Recognizr</h1>
					<span class="ml-3 text-sm text-gray-500">Face Recognition App</span>
				</div>
				<div class="flex items-center space-x-4">
					<div class="text-sm text-gray-500">
						API: {API_BASE}
					</div>
					<div class="flex items-center space-x-2">
						{#if apiStatus === 'checking'}
							<div class="w-2 h-2 bg-yellow-400 rounded-full animate-pulse"></div>
							<span class="text-sm text-yellow-600">Checking...</span>
						{:else if apiStatus === 'connected'}
							<div class="w-2 h-2 bg-green-400 rounded-full"></div>
							<span class="text-sm text-green-600">Connected</span>
							{#if apiInfo}
								<span class="text-xs text-gray-400">v{apiInfo.version}</span>
							{/if}
						{:else}
							<div class="w-2 h-2 bg-red-400 rounded-full"></div>
							<span class="text-sm text-red-600">Disconnected</span>
							<button
								on:click={checkApiStatus}
								class="text-xs text-blue-600 hover:text-blue-800 underline"
							>
								Retry
							</button>
						{/if}
					</div>
				</div>
			</div>
		</div>
	</header>

	<!-- Main Content -->
	<main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
		<!-- Message Display -->
		{#if message}
			<div class="mb-6 p-4 rounded-md {messageType === 'success' ? 'bg-green-50 text-green-800 border border-green-200' : messageType === 'error' ? 'bg-red-50 text-red-800 border border-red-200' : 'bg-blue-50 text-blue-800 border border-blue-200'}">
				{message}
			</div>
		{/if}

		<!-- Tab Navigation -->
		<div class="mb-8">
			<nav class="flex space-x-8" aria-label="Tabs">
				<button
					class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'enroll' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
					on:click={() => activeTab = 'enroll'}
				>
					Enroll Person
				</button>
				<button
					class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'recognize' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
					on:click={() => activeTab = 'recognize'}
				>
					Recognize Faces
				</button>
				<button
					class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'gallery' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
					on:click={() => activeTab = 'gallery'}
				>
					Gallery
				</button>
				<button
					class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'debug' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
					on:click={() => activeTab = 'debug'}
				>
					Debug Detection
				</button>
			</nav>
		</div>

		<!-- Tab Content -->
		<div class="bg-white shadow rounded-lg">
			<!-- Enroll Tab -->
			{#if activeTab === 'enroll'}
				<div class="p-6">
					<h2 class="text-lg font-medium text-gray-900 mb-4">Enroll a New Person</h2>
					<p class="text-sm text-gray-600 mb-6">
						Upload an image containing exactly one face and provide a name to enroll this person in the system.
					</p>

					<form on:submit|preventDefault={handleEnroll} class="space-y-6">
						<div>
							<label for="enrollName" class="block text-sm font-medium text-gray-700">
								Person's Name
							</label>
							<input
								id="enrollName"
								type="text"
								bind:value={enrollName}
								placeholder="Enter the person's name"
								class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500"
								required
								maxlength="100"
							/>
						</div>

						<div>
							<label for="enrollImage" class="block text-sm font-medium text-gray-700">
								Image File
							</label>
							<input
								id="enrollImage"
								type="file"
								accept="image/*"
								bind:this={enrollFileInput}
								on:change={(e) => handleFileSelect(e, 'enroll')}
								class="mt-1 block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-full file:border-0 file:text-sm file:font-semibold file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100"
								required
							/>
							<p class="mt-1 text-xs text-gray-500">
								Supported formats: JPG, PNG, WebP. Max size: 15MB.
							</p>
						</div>

						<button
							type="submit"
							disabled={isLoading}
							class="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
						>
							{#if isLoading}
								<svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
									<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
									<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
								</svg>
								Enrolling...
							{:else}
								Enroll Person
							{/if}
						</button>
					</form>
				</div>
			{/if}

			<!-- Recognize Tab -->
			{#if activeTab === 'recognize'}
				<div class="p-6">
					<h2 class="text-lg font-medium text-gray-900 mb-4">Recognize Faces</h2>
					<p class="text-sm text-gray-600 mb-6">
						Upload an image to identify all known faces in it.
					</p>

					<form on:submit|preventDefault={handleRecognize} class="space-y-6">
						<div>
							<label for="recognizeImage" class="block text-sm font-medium text-gray-700">
								Image File
							</label>
							<input
								id="recognizeImage"
								type="file"
								accept="image/*"
								bind:this={recognizeFileInput}
								on:change={(e) => handleFileSelect(e, 'recognize')}
								class="mt-1 block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-full file:border-0 file:text-sm file:font-semibold file:bg-green-50 file:text-green-700 hover:file:bg-green-100"
								required
							/>
							<p class="mt-1 text-xs text-gray-500">
								Supported formats: JPG, PNG, WebP. Max size: 15MB.
							</p>
						</div>

						<div class="space-y-3">
							<button
								type="submit"
								disabled={isLoading}
								class="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 disabled:opacity-50 disabled:cursor-not-allowed"
							>
								{#if isLoading}
									<svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
										<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
										<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
									</svg>
									Recognizing...
								{:else}
									Recognize Faces
								{/if}
							</button>

							<!-- Test button for development -->
							{#if recognizeImageUrl && imageLoaded}
								<button
									type="button"
									on:click={addTestResults}
									class="w-full flex justify-center py-2 px-4 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500"
								>
									Add Test Results (Demo)
								</button>
							{/if}

							<!-- Name unknown faces button -->
							{#if recognitionResults.length > 0}
								<button
									type="button"
									on:click={() => isNamingMode = !isNamingMode}
									class="w-full flex justify-center py-2 px-4 border border-blue-300 rounded-md shadow-sm text-sm font-medium text-blue-700 bg-blue-50 hover:bg-blue-100 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
								>
									{isNamingMode ? 'Cancel Naming Mode' : 'Name Unknown Faces'}
								</button>
							{/if}
						</div>
					</form>

					<!-- Recognition Results with Image Overlay -->
					{#if recognizeImageUrl}
						<div class="mt-8">
							<h3 class="text-lg font-medium text-gray-900 mb-4">Recognition Results</h3>
							<div class="relative inline-block max-w-full">
								<!-- Main Image -->
								<img
									bind:this={imageElement}
									src={recognizeImageUrl}
									alt="Uploaded for recognition"
									class="max-w-full h-auto rounded-lg shadow-lg block"
									on:load={handleImageLoad}
								/>

								<!-- Face Overlays -->
								{#if imageLoaded && recognitionResults.length > 0}
									{#each recognitionResults as result, index}
										{#if result.bbox && imageElement}
											{@const imageNaturalWidth = imageElement.naturalWidth}
											{@const imageNaturalHeight = imageElement.naturalHeight}
											{@const imageDisplayWidth = imageElement.offsetWidth}
											{@const imageDisplayHeight = imageElement.offsetHeight}
											{@const scaleX = imageDisplayWidth / imageNaturalWidth}
											{@const scaleY = imageDisplayHeight / imageNaturalHeight}
											{@const x1 = result.bbox[0] * scaleX}
											{@const y1 = result.bbox[1] * scaleY}
											{@const x2 = result.bbox[2] * scaleX}
											{@const y2 = result.bbox[3] * scaleY}
											{@const width = x2 - x1}
											{@const height = y2 - y1}
											{@const percentage = similarityToPercentage(result.similarity)}

											<!-- Bounding Box -->
											<div
												class="absolute border-2 {result.name === 'Unknown' ? 'border-gray-400' : percentage >= 75 ? 'border-green-400' : percentage >= 50 ? 'border-yellow-400' : 'border-red-400'} {isNamingMode && result.name === 'Unknown' ? 'cursor-pointer hover:border-blue-500 hover:bg-blue-100 hover:bg-opacity-20' : ''}"
												style="left: {x1}px; top: {y1}px; width: {width}px; height: {height}px;"
												role={isNamingMode && result.name === 'Unknown' ? 'button' : undefined}
												tabindex={isNamingMode && result.name === 'Unknown' ? 0 : -1}
												on:click={() => result.name === 'Unknown' && result.bbox ? handleFaceClick(index, result.bbox) : null}
												on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && result.name === 'Unknown' && result.bbox ? handleFaceClick(index, result.bbox) : null}
												title={isNamingMode && result.name === 'Unknown' ? 'Click to name this face' : ''}
											></div>

											<!-- Label -->
											<div
												class="absolute px-2 py-1 text-xs font-medium text-white rounded shadow-lg {result.name === 'Unknown' ? 'bg-gray-600' : percentage >= 75 ? 'bg-green-600' : percentage >= 50 ? 'bg-yellow-600' : 'bg-red-600'}"
												style="left: {x1}px; top: {Math.max(0, y1 - 28)}px;"
											>
												<div class="whitespace-nowrap">
													{result.name}
													{#if result.name !== 'Unknown'}
														<span class="ml-1 opacity-90">({percentage.toFixed(0)}%)</span>
													{/if}
												</div>
											</div>
										{/if}
									{/each}
								{/if}
							</div>

							<!-- Results Summary -->
							{#if recognitionResults.length > 0}
								<div class="mt-6 bg-gray-50 rounded-lg p-4">
									<h4 class="font-medium text-gray-900 mb-3">Detection Summary</h4>
									<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
										{#each recognitionResults as result, index}
											{@const percentage = similarityToPercentage(result.similarity)}
											<div class="bg-white rounded p-3 border">
												<div class="flex items-center justify-between">
													<span class="font-medium text-sm">Face #{index + 1}</span>
													{#if result.name === 'Unknown'}
														<span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-gray-100 text-gray-800">
															Unknown
														</span>
													{:else}
														<span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {percentage >= 75 ? 'bg-green-100 text-green-800' : percentage >= 50 ? 'bg-yellow-100 text-yellow-800' : 'bg-red-100 text-red-800'}">
															{percentage >= 75 ? 'High' : percentage >= 50 ? 'Medium' : 'Low'}
														</span>
													{/if}
												</div>
												<div class="mt-1">
													<p class="text-sm text-gray-900">{result.name}</p>
													{#if result.name !== 'Unknown'}
														<p class="text-xs text-gray-500">Confidence: {percentage.toFixed(1)}%</p>
														<p class="text-xs text-gray-400">Raw score: {result.similarity.toFixed(3)}</p>
													{/if}
												</div>
											</div>
										{/each}
									</div>
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{/if}

			<!-- Gallery Tab -->
			{#if activeTab === 'gallery'}
				<div class="p-6">
					<h2 class="text-lg font-medium text-gray-900 mb-4">Enrolled People Gallery</h2>
					<p class="text-sm text-gray-600 mb-6">
						View all people enrolled in the face recognition system.
					</p>

					{#if isLoading}
						<div class="flex justify-center items-center py-12">
							<svg class="animate-spin h-8 w-8 text-blue-600" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
								<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
								<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
							</svg>
							<span class="ml-3 text-gray-600">Loading gallery...</span>
						</div>
					{:else if galleryPeople.length === 0}
						<div class="text-center py-12">
							<div class="mx-auto h-12 w-12 text-gray-400">
								<svg fill="none" viewBox="0 0 24 24" stroke="currentColor">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
								</svg>
							</div>
							<h3 class="mt-2 text-sm font-medium text-gray-900">No people enrolled</h3>
							<p class="mt-1 text-sm text-gray-500">Get started by enrolling someone in the "Enroll Person" tab.</p>
						</div>
					{:else}
						<div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
							{#each galleryPeople as person}
								<div class="bg-white rounded-lg shadow-md overflow-hidden hover:shadow-lg transition-shadow">
									<div class="aspect-square">
										<img
											src="data:image/jpeg;base64,{person.image_base64}"
											alt="{person.name}"
											class="w-full h-full object-cover"
										/>
									</div>
									<div class="p-3">
										<h3 class="text-sm font-medium text-gray-900 truncate" title="{person.name}">
											{person.name}
										</h3>
									</div>
								</div>
							{/each}
						</div>

						<div class="mt-6 text-center">
							<p class="text-sm text-gray-500">
								{galleryPeople.length} {galleryPeople.length === 1 ? 'person' : 'people'} enrolled
							</p>
							<button
								type="button"
								on:click={() => { galleryLoaded = false; loadGallery(); }}
								class="mt-2 text-sm text-blue-600 hover:text-blue-500"
							>
								Refresh Gallery
							</button>
						</div>
					{/if}
				</div>
			{/if}

			<!-- Debug Tab -->
			{#if activeTab === 'debug'}
				<div class="p-6">
					<h2 class="text-lg font-medium text-gray-900 mb-4">Debug Face Detection</h2>
					<p class="text-sm text-gray-600 mb-6">
						Upload an image to see the face detection pipeline in action with visual annotations.
					</p>

					<form on:submit|preventDefault={handleDebug} class="space-y-6">
						<div>
							<label for="debugThreshold" class="block text-sm font-medium text-gray-700">
								Detection Threshold
							</label>
							<input
								id="debugThreshold"
								type="number"
								bind:value={debugThreshold}
								min="0.1"
								max="1.0"
								step="0.1"
								class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-purple-500 focus:ring-purple-500"
							/>
							<p class="mt-1 text-xs text-gray-500">
								Higher values = more strict detection (0.1 - 1.0)
							</p>
						</div>

						<div>
							<label for="debugImage" class="block text-sm font-medium text-gray-700">
								Image File
							</label>
							<input
								id="debugImage"
								type="file"
								accept="image/*"
								bind:this={debugFileInput}
								on:change={(e) => handleFileSelect(e, 'debug')}
								class="mt-1 block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-full file:border-0 file:text-sm file:font-semibold file:bg-purple-50 file:text-purple-700 hover:file:bg-purple-100"
								required
							/>
							<p class="mt-1 text-xs text-gray-500">
								Supported formats: JPG, PNG, WebP. Max size: 15MB.
							</p>
						</div>

						<button
							type="submit"
							disabled={isLoading}
							class="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-purple-600 hover:bg-purple-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-purple-500 disabled:opacity-50 disabled:cursor-not-allowed"
						>
							{#if isLoading}
								<svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
									<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
									<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
								</svg>
								Processing...
							{:else}
								Generate Debug Image
							{/if}
						</button>
					</form>

					<!-- Debug Image Result -->
					{#if debugImageUrl}
						<div class="mt-8">
							<h3 class="text-lg font-medium text-gray-900 mb-4">Debug Result</h3>
							<div class="border rounded-lg overflow-hidden">
								<img
									src={debugImageUrl}
									alt="Debug result with face detection annotations"
									class="w-full h-auto"
								/>
							</div>
							<div class="mt-4 flex justify-end">
								<a
									href={debugImageUrl}
									download="debug_result.jpg"
									class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-purple-700 bg-purple-100 hover:bg-purple-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-purple-500"
								>
									Download Image
								</a>
							</div>
						</div>
					{/if}
				</div>
			{/if}
		</div>

		<!-- Footer -->
		<footer class="mt-12 text-center text-sm text-gray-500">
			<p>
				Recognizr Face Recognition App -
				<a href="https://github.com/netcodedev/recognizr" class="text-blue-600 hover:text-blue-500">View on GitHub</a>
			</p>
		</footer>
	</main>

	<!-- Naming Dialog Modal -->
	{#if showNameDialog}
		<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div class="bg-white rounded-lg p-6 max-w-md w-full mx-4">
				<h3 class="text-lg font-medium text-gray-900 mb-4">Name This Face</h3>
				<div class="mb-4">
					<label for="newFaceName" class="block text-sm font-medium text-gray-700 mb-2">
						Person's Name
					</label>
					<input
						id="newFaceName"
						type="text"
						bind:value={newFaceName}
						placeholder="Enter the person's name"
						class="w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500"
						maxlength="100"
						on:keydown={(e) => e.key === 'Enter' && saveFaceName()}
					/>
				</div>
				<div class="flex justify-end space-x-3">
					<button
						type="button"
						on:click={cancelNaming}
						class="px-4 py-2 border border-gray-300 rounded-md text-sm font-medium text-gray-700 bg-white hover:bg-gray-50"
					>
						Cancel
					</button>
					<button
						type="button"
						on:click={saveFaceName}
						disabled={!newFaceName.trim() || isLoading}
						class="px-4 py-2 border border-transparent rounded-md text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
					>
						{isLoading ? 'Saving...' : 'Save Name'}
					</button>
				</div>
			</div>
		</div>
	{/if}
</div>
