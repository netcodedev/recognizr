<script lang="ts">
	import { RecognizrAPI, validateImageFile, formatSimilarity, getConfidenceLevel, API_BASE, type RecognitionResult } from '$lib/api';

	// API instance
	const api = new RecognizrAPI();

	// Component state
	let activeTab: 'enroll' | 'recognize' | 'debug' = 'enroll';
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

	// Debug form state
	let debugFile: File | null = null;
	let debugFileInput: HTMLInputElement;
	let debugThreshold = 0.6;
	let debugImageUrl = '';

	// API status
	let apiStatus: 'checking' | 'connected' | 'disconnected' = 'checking';
	let apiInfo: {status: string, service: string, version: string} | null = null;

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
					</form>

					<!-- Recognition Results -->
					{#if recognitionResults.length > 0}
						<div class="mt-8">
							<h3 class="text-lg font-medium text-gray-900 mb-4">Recognition Results</h3>
							<div class="space-y-3">
								{#each recognitionResults as result, index}
									<div class="bg-gray-50 rounded-lg p-4">
										<div class="flex justify-between items-start">
											<div>
												<h4 class="font-medium text-gray-900">Face #{index + 1}</h4>
												<p class="text-sm text-gray-600">
													Name: <span class="font-medium">{result.name}</span>
												</p>
												<p class="text-sm text-gray-600">
													Similarity: <span class="font-medium">{formatSimilarity(result.similarity)}</span>
												</p>
												{#if result.bbox}
													<p class="text-xs text-gray-500 mt-1">
														Bounding box: [{result.bbox.map(n => n.toFixed(1)).join(', ')}]
													</p>
												{/if}
											</div>
											<div class="text-right">
												{#if result.name === 'Unknown'}
													<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800">
														Unknown
													</span>
												{:else}
													{@const level = getConfidenceLevel(result.similarity)}
													<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {level === 'high' ? 'bg-green-100 text-green-800' : level === 'medium' ? 'bg-yellow-100 text-yellow-800' : 'bg-red-100 text-red-800'}">
														{level === 'high' ? 'High' : level === 'medium' ? 'Medium' : 'Low'} Confidence
													</span>
												{/if}
											</div>
										</div>
									</div>
								{/each}
							</div>
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
				<a href="https://github.com/your-repo" class="text-blue-600 hover:text-blue-500">View on GitHub</a>
			</p>
		</footer>
	</main>
</div>
