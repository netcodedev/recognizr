<script lang="ts">
	import { getAppContext } from '$lib/context';
	import { validateImageFile } from '$lib/api';

	const { api, showMessage } = getAppContext();

	// Debug state
	let debugFile: File | null = $state(null);
	let debugImageUrl: string | null = $state(null);
	let debugLoading = $state(false);
	let debugResultImageUrl: string | null = $state(null);
	let threshold = $state(0.5);

	// Handle file selection
	function handleDebugFileSelect(event: Event) {
		const target = event.target as HTMLInputElement;
		const file = target.files?.[0];

		if (!file) return;

		const validationError = validateImageFile(file);
		if (validationError) {
			showMessage(validationError, 'error');
			return;
		}

		debugFile = file;
		debugImageUrl = URL.createObjectURL(file);
		debugResultImageUrl = null;
	}

	// Run debug detection
	async function runDebugDetection() {
		if (!debugFile) {
			showMessage('Please select an image first', 'error');
			return;
		}

		debugLoading = true;
		try {
			const imageBlob = await api.debug(debugFile, threshold);

			// Clean up previous result image URL
			if (debugResultImageUrl) {
				URL.revokeObjectURL(debugResultImageUrl);
			}

			// Create new URL for the debug result image
			debugResultImageUrl = URL.createObjectURL(imageBlob);
			showMessage('Debug detection completed! Check the annotated image below.', 'success');
		} catch (error: any) {
			console.error('Debug detection failed:', error);
			showMessage(`Debug detection failed: ${error.message || 'Unknown error'}`, 'error');
		} finally {
			debugLoading = false;
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
			debugFile = file;
			debugImageUrl = URL.createObjectURL(file);
			debugResultImageUrl = null;
		}
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
	}
</script>

<div class="space-y-8">
	<div class="text-center">
		<h1 class="text-3xl font-bold text-green-400 mb-4 font-mono">// DEBUG DETECTOR</h1>
		<p class="text-green-300 font-mono opacity-80">
			> Test face detection with adjustable confidence threshold
		</p>
	</div>

	<div class="cyber-card rounded-lg p-8 max-w-4xl mx-auto">
		<!-- File Upload Area -->
		<div class="mb-6">
			<label for="debug-file" class="block text-sm font-mono text-green-400 mb-2">
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
					id="debug-file"
					accept="image/*"
					onchange={handleDebugFileSelect}
					class="hidden"
				/>
				<label for="debug-file" class="cursor-pointer">
					{#if debugImageUrl}
						<img
							src={debugImageUrl}
							alt=""
							class="max-w-full max-h-96 mx-auto rounded-lg shadow-lg"
						/>
						<p class="mt-4 text-sm text-green-300 font-mono">
							Click to change image or drag & drop a new one
						</p>
					{:else}
						<div class="text-green-400 text-4xl mb-4">🔧</div>
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

		<!-- Threshold Control -->
		<div class="mb-6">
			<label for="threshold" class="block text-sm font-mono text-green-400 mb-2">
				> DETECTION_THRESHOLD: {threshold.toFixed(2)}
			</label>
			<input
				type="range"
				id="threshold"
				bind:value={threshold}
				min="0.1"
				max="1.0"
				step="0.05"
				class="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer slider"
			/>
			<div class="flex justify-between text-xs text-green-300/60 font-mono mt-1">
				<span>0.1 (More faces)</span>
				<span>1.0 (Fewer faces)</span>
			</div>
		</div>

		<!-- Debug Button -->
		<button
			onclick={runDebugDetection}
			disabled={!debugFile || debugLoading}
			class="w-full cyber-button px-6 py-3 rounded-lg font-mono text-green-400 border border-green-400/30 hover:border-green-400/50 disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-300 mb-6"
		>
			{#if debugLoading}
				<div class="flex items-center justify-center space-x-2">
					<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-green-400"></div>
					<span>> DEBUGGING...</span>
				</div>
			{:else}
				> RUN_DEBUG_DETECTION
			{/if}
		</button>

		<!-- Debug Results -->
		{#if debugResultImageUrl}
			<div class="space-y-6">
				<!-- Debug Result Image -->
				<div class="bg-green-900/20 border border-green-400/30 rounded-lg p-4">
					<h3 class="text-sm font-mono text-green-400 mb-4">> DEBUG_RESULT</h3>
					<div class="text-center">
						<img
							src={debugResultImageUrl}
							alt=""
							class="max-w-full h-auto rounded-lg shadow-lg mx-auto"
						/>
					</div>
					<div class="mt-4 text-xs text-green-300/60 font-mono text-center">
						Debug image with face detection annotations (threshold: {threshold.toFixed(2)})
					</div>
				</div>
			</div>
		{/if}

		<!-- Info Box -->
		<div class="mt-6 bg-green-900/20 border border-green-400/30 rounded-lg p-4">
			<p class="text-xs text-green-300 font-mono mb-2">
				<span class="text-green-400">INFO:</span> This debug tool shows raw face detection results with adjustable confidence threshold.
				Lower thresholds detect more faces but may include false positives.
			</p>
			<p class="text-xs text-yellow-300 font-mono">
				<span class="text-yellow-400">TIP:</span> Use this to fine-tune detection sensitivity and troubleshoot recognition issues.
			</p>
		</div>
	</div>
</div>

<style>
	.slider::-webkit-slider-thumb {
		appearance: none;
		height: 20px;
		width: 20px;
		border-radius: 50%;
		background: #22c55e;
		cursor: pointer;
		border: 2px solid #1f2937;
	}

	.slider::-moz-range-thumb {
		height: 20px;
		width: 20px;
		border-radius: 50%;
		background: #22c55e;
		cursor: pointer;
		border: 2px solid #1f2937;
	}
</style>
