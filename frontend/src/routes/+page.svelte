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

<div class="min-h-screen">
	<!-- Header -->
	<header class="cyber-border border-b-2 border-cyan-400/30">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex justify-between items-center py-6">
				<div class="flex items-center">
					<h1 class="text-4xl font-bold gradient-text">RECOGNIZR</h1>
					<span class="ml-4 text-sm text-cyan-300 font-mono">// NEURAL FACE RECOGNITION</span>
				</div>
				<div class="flex items-center space-x-4">
					<div class="text-sm text-cyan-400 font-mono">
						API: {API_BASE}
					</div>
					<div class="flex items-center space-x-2">
						{#if apiStatus === 'checking'}
							<div class="w-3 h-3 bg-yellow-400 rounded-full animate-pulse cyber-glow"></div>
							<span class="text-sm text-yellow-400 font-mono">SCANNING...</span>
						{:else if apiStatus === 'connected'}
							<div class="w-3 h-3 bg-green-400 rounded-full cyber-glow"></div>
							<span class="text-sm text-green-400 font-mono">ONLINE</span>
							{#if apiInfo}
								<span class="text-xs text-cyan-300 font-mono">v{apiInfo.version}</span>
							{/if}
						{:else}
							<div class="w-3 h-3 bg-red-400 rounded-full cyber-glow-pink"></div>
							<span class="text-sm text-red-400 font-mono">OFFLINE</span>
							<button
								on:click={checkApiStatus}
								class="text-xs text-cyan-400 hover:text-pink-400 underline font-mono transition-colors"
							>
								RETRY
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
			<div class="mb-6 p-4 rounded-lg cyber-card {messageType === 'success' ? 'border-green-400 text-green-400' : messageType === 'error' ? 'border-red-400 text-red-400' : 'border-cyan-400 text-cyan-400'} font-mono">
				<span class="text-xs opacity-70">[{messageType.toUpperCase()}]</span> {message}
			</div>
		{/if}

		<!-- Tab Navigation -->
		<div class="mb-8">
			<nav class="flex space-x-8" aria-label="Tabs">
				<button
					class="py-3 px-4 border-b-2 font-mono text-sm transition-all duration-300 {activeTab === 'enroll' ? 'border-cyan-400 text-cyan-400 cyber-glow' : 'border-transparent text-gray-400 hover:text-cyan-300 hover:border-cyan-500/50'}"
					on:click={() => activeTab = 'enroll'}
				>
					// ENROLL
				</button>
				<button
					class="py-3 px-4 border-b-2 font-mono text-sm transition-all duration-300 {activeTab === 'recognize' ? 'border-pink-400 text-pink-400 cyber-glow-pink' : 'border-transparent text-gray-400 hover:text-pink-300 hover:border-pink-500/50'}"
					on:click={() => activeTab = 'recognize'}
				>
					// RECOGNIZE
				</button>
				<button
					class="py-3 px-4 border-b-2 font-mono text-sm transition-all duration-300 {activeTab === 'gallery' ? 'border-purple-400 text-purple-400 cyber-glow-purple' : 'border-transparent text-gray-400 hover:text-purple-300 hover:border-purple-500/50'}"
					on:click={() => activeTab = 'gallery'}
				>
					// GALLERY
				</button>
				<button
					class="py-3 px-4 border-b-2 font-mono text-sm transition-all duration-300 {activeTab === 'debug' ? 'border-yellow-400 text-yellow-400' : 'border-transparent text-gray-400 hover:text-yellow-300 hover:border-yellow-500/50'}"
					on:click={() => activeTab = 'debug'}
				>
					// DEBUG
				</button>
			</nav>
		</div>

		<!-- Tab Content -->
		<div class="bg-gray-900 rounded-lg border border-cyan-400/20">
			<!-- Enroll Tab -->
			{#if activeTab === 'enroll'}
				<div class="p-8">
					<h2 class="text-xl font-bold text-cyan-400 mb-4 font-mono">// NEURAL ENROLLMENT PROTOCOL</h2>
					<p class="text-sm text-cyan-300 mb-8 font-mono opacity-80">
						> Upload biometric data containing exactly one facial pattern for neural network training
					</p>

					<form on:submit|preventDefault={handleEnroll} class="space-y-8">
						<div>
							<label for="enrollName" class="block text-sm font-mono text-cyan-400 mb-2">
								> SUBJECT_ID
							</label>
							<input
								id="enrollName"
								type="text"
								bind:value={enrollName}
								placeholder="Enter subject identifier..."
								class="cyber-input mt-1 block w-full rounded-md font-mono placeholder-cyan-600"
								required
								maxlength="100"
							/>
						</div>

						<div>
							<label for="enrollImage" class="block text-sm font-mono text-cyan-400 mb-2">
								> BIOMETRIC_DATA
							</label>
							<input
								id="enrollImage"
								type="file"
								accept="image/*"
								bind:this={enrollFileInput}
								on:change={(e) => handleFileSelect(e, 'enroll')}
								class="cyber-input mt-1 block rounded-md w-full text-sm font-mono file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-mono file:bg-cyan-900/50 file:text-cyan-400 hover:file:bg-cyan-800/50 file:cyber-glow"
								required
							/>
							<p class="mt-2 text-xs text-cyan-500 font-mono opacity-70">
								// Supported: JPG, PNG, WebP | Max: 15MB
							</p>
						</div>

						<button
							type="submit"
							disabled={isLoading}
							class="cyber-button w-full flex justify-center py-4 px-6 rounded-md text-sm font-mono text-cyan-400 disabled:opacity-50 disabled:cursor-not-allowed"
						>
							{#if isLoading}
								<svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-cyan-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
									<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
									<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
								</svg>
								> PROCESSING...
							{:else}
								> INITIATE_ENROLLMENT
							{/if}
						</button>
					</form>
				</div>
			{/if}

			<!-- Recognize Tab -->
			{#if activeTab === 'recognize'}
				<div class="p-8">
					<h2 class="text-xl font-bold text-pink-400 mb-4 font-mono">// FACIAL RECOGNITION SCANNER</h2>
					<p class="text-sm text-pink-300 mb-8 font-mono opacity-80">
						> Analyze biometric patterns and match against neural database
					</p>

					<form on:submit|preventDefault={handleRecognize} class="space-y-8">
						<div>
							<label for="recognizeImage" class="block text-sm font-mono text-pink-400 mb-2">
								> SCAN_TARGET
							</label>
							<input
								id="recognizeImage"
								type="file"
								accept="image/*"
								on:change={(e) => handleFileSelect(e, 'recognize')}
								class="cyber-input rounded-md mt-1 block w-full text-sm font-mono file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-mono file:bg-pink-900/50 file:text-pink-400 hover:file:bg-pink-800/50 file:cyber-glow-pink"
								required
							/>
							<p class="mt-2 text-xs text-pink-500 font-mono opacity-70">
								// Multi-face detection enabled | Max: 15MB
							</p>
						</div>

						<div class="space-y-4">
							<button
								type="submit"
								disabled={isLoading}
								class="cyber-button w-full flex justify-center py-4 px-6 rounded-md text-sm font-mono text-pink-400 disabled:opacity-50 disabled:cursor-not-allowed"
							>
								{#if isLoading}
									<svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-pink-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
										<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
										<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
									</svg>
									> SCANNING...
								{:else}
									> INITIATE_SCAN
								{/if}
							</button>

							<!-- Name unknown faces button -->
							{#if recognitionResults.length > 0}
								<button
									type="button"
									on:click={() => isNamingMode = !isNamingMode}
									class="cyber-button w-full flex justify-center py-3 px-4 rounded-md text-sm font-mono text-purple-400 border-purple-400/30"
								>
									{isNamingMode ? '> CANCEL_NAMING' : '> NAME_UNKNOWNS'}
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
											<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
											<div
												class="absolute border-2 transition-all duration-300 {result.name === 'Unknown' ? 'border-gray-400 shadow-lg shadow-gray-400/20' : percentage >= 75 ? 'border-green-400 shadow-lg shadow-green-400/30 cyber-glow' : percentage >= 50 ? 'border-yellow-400 shadow-lg shadow-yellow-400/30' : 'border-red-400 shadow-lg shadow-red-400/30'} {isNamingMode && result.name === 'Unknown' ? 'cursor-pointer hover:border-cyan-400 hover:shadow-cyan-400/40 hover:bg-cyan-400/10' : ''}"
												style="left: {x1}px; top: {y1}px; width: {width}px; height: {height}px;"
												role={isNamingMode && result.name === 'Unknown' ? 'button' : undefined}
												tabindex={isNamingMode && result.name === 'Unknown' ? 0 : -1}
												on:click={() => result.name === 'Unknown' && result.bbox ? handleFaceClick(index, result.bbox) : null}
												on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && result.name === 'Unknown' && result.bbox ? handleFaceClick(index, result.bbox) : null}
												title={isNamingMode && result.name === 'Unknown' ? 'Click to name this face' : ''}
											></div>

											<!-- Label -->
											<div
												class="absolute px-3 py-2 text-xs font-mono font-bold text-black rounded-md backdrop-blur-sm border transition-all duration-300 {result.name === 'Unknown' ? 'bg-gray-400/90 border-gray-400' : percentage >= 75 ? 'bg-green-400/90 border-green-400 cyber-glow' : percentage >= 50 ? 'bg-yellow-400/90 border-yellow-400' : 'bg-red-400/90 border-red-400'}"
												style="left: {x1}px; top: {Math.max(0, y1 - 36)}px;"
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

							<!-- Results Summary -->
							{#if recognitionResults.length > 0}
								<div class="mt-8 cyber-card rounded-lg p-6">
									<h4 class="font-bold text-pink-400 mb-6 font-mono">// DETECTION_ANALYSIS</h4>
									<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
										{#each recognitionResults as result, index}
											{@const percentage = similarityToPercentage(result.similarity)}
											<div class="cyber-border rounded-lg p-4 bg-black/20">
												<div class="flex items-center justify-between mb-3">
													<span class="font-mono text-sm text-pink-300">TARGET_{index + 1}</span>
													{#if result.name === 'Unknown'}
														<span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-mono bg-gray-700/50 text-gray-400 border border-gray-600/30">
															UNKNOWN
														</span>
													{:else}
														<span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-mono border {percentage >= 75 ? 'bg-green-900/50 text-green-400 border-green-500/30' : percentage >= 50 ? 'bg-yellow-900/50 text-yellow-400 border-yellow-500/30' : 'bg-red-900/50 text-red-400 border-red-500/30'}">
															{percentage >= 75 ? 'HIGH' : percentage >= 50 ? 'MED' : 'LOW'}
														</span>
													{/if}
												</div>
												<div class="space-y-1">
													<p class="text-sm font-mono text-pink-200">{result.name.toUpperCase()}</p>
													{#if result.name !== 'Unknown'}
														<p class="text-xs font-mono text-pink-400/70">CONF: {percentage.toFixed(1)}%</p>
														<p class="text-xs font-mono text-pink-500/50">RAW: {result.similarity.toFixed(3)}</p>
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
				<div class="p-8">
					<h2 class="text-xl font-bold text-purple-400 mb-4 font-mono">// NEURAL DATABASE ARCHIVE</h2>
					<p class="text-sm text-purple-300 mb-8 font-mono opacity-80">
						> Displaying all registered biometric patterns in the system
					</p>

					{#if isLoading}
						<div class="flex justify-center items-center py-16">
							<svg class="animate-spin h-10 w-10 text-purple-400 cyber-glow-purple" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
								<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
								<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
							</svg>
							<span class="ml-4 text-purple-400 font-mono">ACCESSING DATABASE...</span>
						</div>
					{:else if galleryPeople.length === 0}
						<div class="text-center py-16">
							<div class="mx-auto h-16 w-16 text-purple-400/50 cyber-glow-purple">
								<svg fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
									<path stroke-linecap="round" stroke-linejoin="round" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
								</svg>
							</div>
							<h3 class="mt-4 text-lg font-mono text-purple-400">DATABASE_EMPTY</h3>
							<p class="mt-2 text-sm text-purple-300 font-mono opacity-70">// No biometric patterns found. Initialize enrollment protocol.</p>
						</div>
					{:else}
						<div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-6">
							{#each galleryPeople as person}
								<div class="cyber-card rounded-lg overflow-hidden hover:cyber-glow-purple transition-all duration-300 transform hover:scale-105">
									<div class="aspect-square relative">
										<img
											src="data:image/jpeg;base64,{person.image_base64}"
											alt="{person.name}"
											class="w-full h-full object-cover"
										/>
										<div class="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent"></div>
									</div>
									<div class="p-4 bg-gradient-to-r from-purple-900/20 to-cyan-900/20">
										<h3 class="text-sm font-mono text-purple-300 truncate" title="{person.name}">
											> {person.name.toUpperCase()}
										</h3>
									</div>
								</div>
							{/each}
						</div>

						<div class="mt-8 text-center">
							<p class="text-sm text-purple-400 font-mono">
								// {galleryPeople.length} PATTERN{galleryPeople.length === 1 ? '' : 'S'}_REGISTERED
							</p>
							<button
								type="button"
								on:click={() => { galleryLoaded = false; loadGallery(); }}
								class="cyber-button mt-4 px-6 py-2 text-sm font-mono text-purple-400 border-purple-400/30"
							>
								> REFRESH_DATABASE
							</button>
						</div>
					{/if}
				</div>
			{/if}

			<!-- Debug Tab -->
			{#if activeTab === 'debug'}
				<div class="p-8">
					<h2 class="text-xl font-bold text-yellow-400 mb-4 font-mono">// DETECTION PIPELINE DEBUG</h2>
					<p class="text-sm text-yellow-300 mb-8 font-mono opacity-80">
						> Visualize neural network detection layers with confidence annotations
					</p>

					<form on:submit|preventDefault={handleDebug} class="space-y-8">
						<div>
							<label for="debugThreshold" class="block text-sm font-mono text-yellow-400 mb-2">
								> CONFIDENCE_THRESHOLD
							</label>
							<input
								id="debugThreshold"
								type="number"
								bind:value={debugThreshold}
								min="0.1"
								max="1.0"
								step="0.1"
								class="cyber-input mt-1 block w-full rounded-md font-mono"
							/>
							<p class="mt-2 text-xs text-yellow-500 font-mono opacity-70">
								// Higher values = stricter detection (0.1 - 1.0)
							</p>
						</div>

						<div>
							<label for="debugImage" class="block text-sm font-mono text-yellow-400 mb-2">
								> DEBUG_TARGET
							</label>
							<input
								id="debugImage"
								type="file"
								accept="image/*"
								bind:this={debugFileInput}
								on:change={(e) => handleFileSelect(e, 'debug')}
								class="cyber-input rounded-md mt-1 block w-full text-sm font-mono file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-mono file:bg-yellow-900/50 file:text-yellow-400 hover:file:bg-yellow-800/50"
								required
							/>
							<p class="mt-2 text-xs text-yellow-500 font-mono opacity-70">
								// Pipeline visualization enabled | Max: 15MB
							</p>
						</div>

						<button
							type="submit"
							disabled={isLoading}
							class="cyber-button w-full flex justify-center py-4 px-6 rounded-md text-sm font-mono text-yellow-400 disabled:opacity-50 disabled:cursor-not-allowed"
						>
							{#if isLoading}
								<svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-yellow-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
									<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
									<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
								</svg>
								> ANALYZING...
							{:else}
								> GENERATE_DEBUG
							{/if}
						</button>
					</form>

					<!-- Debug Image Result -->
					{#if debugImageUrl}
						<div class="mt-10">
							<h3 class="text-lg font-bold text-yellow-400 mb-6 font-mono">// PIPELINE_OUTPUT</h3>
							<div class="cyber-border rounded-lg overflow-hidden">
								<img
									src={debugImageUrl}
									alt="Debug result with face detection annotations"
									class="w-full h-auto"
								/>
							</div>
							<div class="mt-6 flex justify-end">
								<a
									href={debugImageUrl}
									download="debug_result.jpg"
									class="cyber-button inline-flex items-center px-6 py-3 text-sm font-mono text-yellow-400 border-yellow-400/30"
								>
									> DOWNLOAD_OUTPUT
								</a>
							</div>
						</div>
					{/if}
				</div>
			{/if}
		</div>

		<!-- Footer -->
		<footer class="mt-16 text-center text-sm text-cyan-500/70 font-mono">
			<p>
				// RECOGNIZR v2.0 - NEURAL BIOMETRIC SYSTEM //
				<a href="https://github.com/netcodedev/recognizr" class="text-cyan-400 hover:text-pink-400 transition-colors cyber-glow">
					[SOURCE_CODE]
				</a>
			</p>
		</footer>
	</main>

	<!-- Naming Dialog Modal -->
	{#if showNameDialog}
		<div class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50">
			<div class="cyber-card rounded-lg p-8 max-w-md w-full mx-4 neon-border">
				<h3 class="text-xl font-bold text-purple-400 mb-6 font-mono">// SUBJECT_IDENTIFICATION</h3>
				<div class="mb-6">
					<label for="newFaceName" class="block text-sm font-mono text-purple-300 mb-3">
						> ASSIGN_IDENTIFIER
					</label>
					<input
						id="newFaceName"
						type="text"
						bind:value={newFaceName}
						placeholder="Enter subject name..."
						class="cyber-input w-full rounded-md font-mono placeholder-purple-600"
						maxlength="100"
						on:keydown={(e) => e.key === 'Enter' && saveFaceName()}
					/>
				</div>
				<div class="flex justify-end space-x-4">
					<button
						type="button"
						on:click={cancelNaming}
						class="cyber-button px-6 py-3 rounded-md text-sm font-mono text-gray-400 border-gray-500/30"
					>
						> ABORT
					</button>
					<button
						type="button"
						on:click={saveFaceName}
						disabled={!newFaceName.trim() || isLoading}
						class="cyber-button px-6 py-3 rounded-md text-sm font-mono text-purple-400 border-purple-400/30 disabled:opacity-50 disabled:cursor-not-allowed"
					>
						{isLoading ? '> PROCESSING...' : '> CONFIRM'}
					</button>
				</div>
			</div>
		</div>
	{/if}
</div>
