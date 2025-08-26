<script lang="ts">
	import { onMount } from 'svelte';
	import { getAppContext } from '$lib/context';

	const { getGallery, loadGallery } = getAppContext();

	let gallery = $derived(getGallery());

	// Load gallery on mount
	onMount(() => {
		loadGallery();
	});
</script>

<div class="space-y-8">
	<div class="text-center">
		<h1 class="text-3xl font-bold text-green-400 mb-4 font-mono">// FACE GALLERY</h1>
		<p class="text-green-300 font-mono opacity-80">
			> Browse all enrolled faces in the recognition database
		</p>
	</div>

	<div class="cyber-card rounded-lg p-8">
		<div class="flex justify-between items-center mb-6">
			<h2 class="text-xl font-mono text-green-400">
				> ENROLLED_FACES ({gallery.length})
			</h2>
			<button
				onclick={loadGallery}
				class="cyber-button px-4 py-2 rounded-md text-xs font-mono text-green-400 border-green-400/30 hover:border-green-400/50"
			>
				> REFRESH
			</button>
		</div>

		{#if gallery.length === 0}
			<div class="text-center py-12">
				<div class="text-green-400 text-6xl mb-4">👤</div>
				<h3 class="text-lg font-mono text-green-300 mb-2">No faces enrolled yet</h3>
				<p class="text-green-300/60 font-mono text-sm mb-6">
					Start by enrolling some faces to build your recognition database
				</p>
				<a
					href="/"
					class="cyber-button px-6 py-3 rounded-lg font-mono text-green-400 border border-green-400/30 hover:border-green-400/50 transition-all duration-300 inline-block"
				>
					> ENROLL_FIRST_FACE
				</a>
			</div>
		{:else}
			<div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6">
				{#each gallery as person}
					<div class="group">
						<div class="aspect-square bg-gray-800 rounded-lg overflow-hidden border border-green-400/20 hover:border-green-400/50 transition-all duration-300 relative">
							<img
								src="data:image/jpeg;base64,{person.image_base64}"
								alt={person.name}
								class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
								loading="lazy"
							/>
							
							<!-- Overlay with name -->
							<div class="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300">
								<div class="absolute bottom-0 left-0 right-0 p-3">
									<div class="text-green-400 font-mono text-sm font-bold truncate">
										{person.name}
									</div>
								</div>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>
