<script lang="ts">
	import { getAppContext } from '$lib/context';
	import { validateImageFile } from '$lib/api';

	const { api, showMessage } = getAppContext();

	// Enroll state
	let enrollFile: File | null = $state(null);
	let enrollImageUrl: string | null = $state(null);
	let enrollName = $state('');
	let enrollLoading = $state(false);

	// Handle file selection
	function handleEnrollFileSelect(event: Event) {
		const target = event.target as HTMLInputElement;
		const file = target.files?.[0];

		if (!file) return;

		const validationError = validateImageFile(file);
		if (validationError) {
			showMessage(validationError, 'error');
			return;
		}

		enrollFile = file;
		enrollImageUrl = URL.createObjectURL(file);
		enrollName = '';
	}

	// Enroll face
	async function enrollFace() {
		if (!enrollFile || !enrollName.trim()) {
			showMessage('Please select an image and enter a name', 'error');
			return;
		}

		enrollLoading = true;
		try {
			await api.enroll(enrollName.trim(), enrollFile);
			showMessage(`Successfully enrolled ${enrollName}!`, 'success');
			
			// Reset form
			enrollFile = null;
			enrollImageUrl = null;
			enrollName = '';
			
			// Reset file input
			const fileInput = document.getElementById('enroll-file') as HTMLInputElement;
			if (fileInput) fileInput.value = '';
		} catch (error: any) {
			console.error('Enrollment failed:', error);
			showMessage(`Enrollment failed: ${error.message || 'Unknown error'}`, 'error');
		} finally {
			enrollLoading = false;
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
			enrollFile = file;
			enrollImageUrl = URL.createObjectURL(file);
			enrollName = '';
		}
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
	}
</script>

<div class="space-y-8">
	<div class="text-center">
		<h1 class="text-3xl font-bold text-green-400 mb-4 font-mono">// ENROLL NEW FACE</h1>
		<p class="text-green-300 font-mono opacity-80">
			> Upload an image and assign a name to add a new person to the recognition database
		</p>
	</div>

	<div class="cyber-card rounded-lg p-8 max-w-2xl mx-auto">
		<!-- File Upload Area -->
		<div class="mb-6">
			<label for="enroll-file" class="block text-sm font-mono text-green-400 mb-2">
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
					id="enroll-file"
					accept="image/*"
					onchange={handleEnrollFileSelect}
					class="hidden"
				/>
				<label for="enroll-file" class="cursor-pointer">
					{#if enrollImageUrl}
						<img
							src={enrollImageUrl}
							alt="Selected for enrollment"
							class="max-w-full max-h-64 mx-auto rounded-lg shadow-lg"
						/>
						<p class="mt-4 text-sm text-green-300 font-mono">
							Click to change image or drag & drop a new one
						</p>
					{:else}
						<div class="text-green-400 text-4xl mb-4">📷</div>
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

		<!-- Name Input -->
		<div class="mb-6">
			<label for="enroll-name" class="block text-sm font-mono text-green-400 mb-2">
				> PERSON_NAME
			</label>
			<input
				type="text"
				id="enroll-name"
				bind:value={enrollName}
				placeholder="Enter person's name..."
				class="w-full px-4 py-3 bg-gray-800 border border-green-400/30 rounded-lg text-green-100 placeholder-green-300/50 focus:border-green-400 focus:outline-none font-mono"
				disabled={enrollLoading}
			/>
		</div>

		<!-- Enroll Button -->
		<button
			onclick={enrollFace}
			disabled={!enrollFile || !enrollName.trim() || enrollLoading}
			class="w-full cyber-button px-6 py-3 rounded-lg font-mono text-green-400 border border-green-400/30 hover:border-green-400/50 disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-300"
		>
			{#if enrollLoading}
				<div class="flex items-center justify-center space-x-2">
					<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-green-400"></div>
					<span>> ENROLLING...</span>
				</div>
			{:else}
				> ENROLL_FACE
			{/if}
		</button>

		<!-- Info Box -->
		<div class="mt-6 bg-green-900/20 border border-green-400/30 rounded-lg p-4">
			<p class="text-xs text-green-300 font-mono mb-2">
				<span class="text-green-400">INFO:</span> The system will detect faces in the image and enroll the most prominent one.
				For best results, use clear, well-lit photos with a single person.
			</p>
			<p class="text-xs text-yellow-300 font-mono">
				<span class="text-yellow-400">TIP:</span> Names are case-sensitive and will be used for recognition results.
			</p>
		</div>
	</div>
</div>
