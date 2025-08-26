<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { RecognizrAPI, type GalleryPerson } from '$lib/api';
	import { setAppContext } from '$lib/context';
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();

	// API instance
	let api = new RecognizrAPI();

	// Global state
	let apiStatus = $state<'checking' | 'connected' | 'disconnected'>('checking');
	let apiInfo = $state<{status: string, service: string, version: string} | null>(null);
	let gallery = $state<GalleryPerson[]>([]);

	// Message system
	let message = $state('');
	let messageType = $state<'success' | 'error' | 'info'>('info');
	let messageVisible = $state(false);

	export function showMessage(text: string, type: 'success' | 'error' | 'info' = 'info') {
		message = text;
		messageType = type;
		messageVisible = true;
		setTimeout(() => {
			messageVisible = false;
		}, 5000);
	}

	// Check API status on mount
	onMount(async () => {
		try {
			const health = await api.healthCheck();
			if (health) {
				apiStatus = 'connected';
				apiInfo = health;
			} else {
				apiStatus = 'disconnected';
			}
		} catch {
			apiStatus = 'disconnected';
		}
	});

	// Load gallery data
	async function loadGallery() {
		try {
			gallery = await api.getGallery();
		} catch (error: any) {
			console.error('Failed to load gallery:', error);
			showMessage(`Failed to load gallery: ${error.message || 'Unknown error'}`, 'error');
		}
	}

	// Navigation items
	const navItems = [
		{ href: '/', label: 'ENROLL', icon: '👤' },
		{ href: '/recognize', label: 'RECOGNIZE', icon: '🔍' },
		{ href: '/gallery', label: 'GALLERY', icon: '📸' },
		{ href: '/debug', label: 'DEBUG', icon: '🔧' },
		{ href: '/google-photos', label: 'GOOGLE_PHOTOS', icon: '📷' }
	];

	let currentPath = $derived($page.url.pathname);

	// Set context for child components
	setAppContext({
		api,
		showMessage,
		getGallery: () => gallery,
		loadGallery
	});
</script>

<div class="min-h-screen">
	<!-- Header -->
	<header class="bg-gray-900 border-b border-green-400/30 shadow-lg">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex justify-between items-center py-4">
				<div class="flex items-center space-x-4">
					<div class="text-2xl font-bold text-green-400 font-mono">
						> RECOGNIZR_
					</div>
					<div class="text-sm text-green-300/60 font-mono">
						{#if apiStatus === 'connected' && apiInfo}
							v{apiInfo.version}
						{:else}
							OFFLINE
						{/if}
					</div>
				</div>

				<!-- API Status -->
				<div class="flex items-center space-x-2">
					<div class="w-2 h-2 rounded-full {apiStatus === 'connected' ? 'bg-green-400 animate-pulse' : apiStatus === 'checking' ? 'bg-yellow-400 animate-pulse' : 'bg-red-400'}"></div>
					<span class="text-xs font-mono {apiStatus === 'connected' ? 'text-green-400' : apiStatus === 'checking' ? 'text-yellow-400' : 'text-red-400'}">
						{apiStatus === 'connected' ? 'CONNECTED' : apiStatus === 'checking' ? 'CHECKING...' : 'DISCONNECTED'}
					</span>
				</div>
			</div>
		</div>
	</header>

	<!-- Navigation -->
	<nav class="bg-gray-800 border-b border-green-400/20">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex space-x-8 overflow-x-auto">
				{#each navItems as item}
					<a
						href={item.href}
						class="flex items-center space-x-2 px-3 py-4 text-sm font-mono transition-colors duration-200 border-b-2 {
							currentPath === item.href
								? 'text-green-400 border-green-400 bg-green-400/10'
								: 'text-green-300/70 border-transparent hover:text-green-300 hover:border-green-400/50'
						}"
					>
						<span class="text-lg">{item.icon}</span>
						<span>{item.label}</span>
					</a>
				{/each}
			</div>
		</div>
	</nav>

	<!-- Message Toast -->
	{#if messageVisible}
		<div class="fixed top-4 right-4 z-50 max-w-sm">
			<div class="rounded-lg shadow-lg border backdrop-blur-sm {
				messageType === 'success' ? 'bg-green-900/90 border-green-400 text-green-100' :
				messageType === 'error' ? 'bg-red-900/90 border-red-400 text-red-100' :
				'bg-blue-900/90 border-blue-400 text-blue-100'
			}">
				<div class="p-4">
					<div class="flex items-start">
						<div class="flex-shrink-0">
							{#if messageType === 'success'}
								<span class="text-green-400">✓</span>
							{:else if messageType === 'error'}
								<span class="text-red-400">✗</span>
							{:else}
								<span class="text-blue-400">ℹ</span>
							{/if}
						</div>
						<div class="ml-3 flex-1">
							<p class="text-sm font-mono">{message}</p>
						</div>
						<div class="ml-4 flex-shrink-0">
							<button
								onclick={() => messageVisible = false}
								class="text-gray-400 hover:text-gray-200 transition-colors"
							>
								<span class="sr-only">Close</span>
								<span class="text-lg">×</span>
							</button>
						</div>
					</div>
				</div>
			</div>
		</div>
	{/if}

	<!-- Main Content -->
	<main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
		{@render children?.()}
	</main>
</div>

<style>
	:global(.cyber-card) {
		background: linear-gradient(135deg, rgba(0, 20, 40, 0.9), rgba(0, 40, 80, 0.8));
		backdrop-filter: blur(10px);
		border: 1px solid rgba(34, 197, 94, 0.3);
	}

	:global(.cyber-button) {
		background: linear-gradient(135deg, rgba(0, 20, 40, 0.8), rgba(0, 40, 80, 0.6));
		backdrop-filter: blur(5px);
		transition: all 0.3s ease;
	}

	:global(.cyber-button:hover) {
		background: linear-gradient(135deg, rgba(34, 197, 94, 0.2), rgba(0, 40, 80, 0.8));
		box-shadow: 0 0 20px rgba(34, 197, 94, 0.3);
	}

	:global(.cyber-glow) {
		box-shadow: 0 0 20px rgba(34, 197, 94, 0.4);
	}

	:global(.neon-border) {
		box-shadow:
			0 0 5px rgba(34, 197, 94, 0.5),
			0 0 10px rgba(34, 197, 94, 0.3),
			0 0 15px rgba(34, 197, 94, 0.2);
	}
</style>

<svelte:head>
	<title>Recognizr - Face Recognition App</title>
	<meta name="description" content="Face recognition application using the Recognizr API" />
	<link rel="icon" href={favicon} />
</svelte:head>
