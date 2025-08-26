<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { GooglePhotosAPI } from '$lib/api';

	let status = 'Processing authentication...';
	let error = '';
	let isLoading = true;

	const googlePhotos = new GooglePhotosAPI();

	onMount(async () => {
		try {
			// Get the authorization code from URL parameters
			const code = $page.url.searchParams.get('code');
			const error_param = $page.url.searchParams.get('error');
			const state = $page.url.searchParams.get('state');

			if (error_param) {
				throw new Error(`Authentication failed: ${error_param}`);
			}

			if (!code) {
				throw new Error('No authorization code received');
			}

			status = 'Exchanging authorization code for access token...';

			// Exchange the code for an access token
			await googlePhotos.exchangeCodeForToken(code);

			status = 'Authentication successful! Redirecting...';

			// Redirect back to the main page with a success indicator
			setTimeout(() => {
				goto('/?google_photos_auth=success');
			}, 2000);

		} catch (err) {
			console.error('OAuth callback error:', err);
			error = err instanceof Error ? err.message : 'Unknown error occurred';
			isLoading = false;
		}
	});
</script>

<svelte:head>
	<title>Google Photos Authentication - Recognizr</title>
</svelte:head>

<main class="min-h-screen bg-gradient-to-br from-gray-900 via-purple-900 to-violet-900 flex items-center justify-center p-4">
	<div class="bg-gray-900 rounded-lg border border-cyan-400/20 p-8 max-w-md w-full text-center">
		<h1 class="text-2xl font-bold text-cyan-400 mb-6 font-mono">// GOOGLE PHOTOS AUTH</h1>
		
		{#if isLoading}
			<div class="space-y-4">
				<!-- Loading spinner -->
				<div class="flex justify-center">
					<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-cyan-400"></div>
				</div>
				
				<p class="text-cyan-300 font-mono text-sm">{status}</p>
			</div>
		{:else if error}
			<div class="space-y-4">
				<div class="text-red-400 text-4xl mb-4">⚠️</div>
				<h2 class="text-xl font-bold text-red-400 font-mono">Authentication Failed</h2>
				<p class="text-red-300 font-mono text-sm">{error}</p>
				<button 
					on:click={() => goto('/')}
					class="cyber-button w-full flex justify-center py-3 px-4 rounded-md text-sm font-mono text-cyan-400 border-cyan-400/30 hover:border-cyan-400/50 transition-colors"
				>
					> RETURN_TO_MAIN
				</button>
			</div>
		{/if}
	</div>
</main>

<style>
	.cyber-button {
		background: linear-gradient(135deg, rgba(6, 182, 212, 0.1) 0%, rgba(147, 51, 234, 0.1) 100%);
		border: 1px solid;
		box-shadow: 0 0 20px rgba(6, 182, 212, 0.3);
		transition: all 0.3s ease;
	}

	.cyber-button:hover {
		box-shadow: 0 0 30px rgba(6, 182, 212, 0.5);
		transform: translateY(-1px);
	}
</style>
