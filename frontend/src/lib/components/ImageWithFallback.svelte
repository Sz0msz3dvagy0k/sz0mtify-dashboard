<script lang="ts">
	import { Disc3, UserRound } from 'lucide-svelte';
	import { initials } from '$lib/format';

	export let src: string | null = null;
	export let fallbackSrc: string | null = null;
	export let alt = '';
	export let kind: 'album' | 'artist' = 'album';
	let failedPrimary = false;
	let failedFallback = false;
	let lastSrc: string | null = null;
	let lastFallbackSrc: string | null = null;

	$: if (src !== lastSrc) {
		lastSrc = src;
		failedPrimary = false;
	}
	$: if (fallbackSrc !== lastFallbackSrc) {
		lastFallbackSrc = fallbackSrc;
		failedFallback = false;
	}
</script>

{#if src && !failedPrimary}
	<img class="art-image" {src} {alt} loading="lazy" on:error={() => (failedPrimary = true)} />
{:else if fallbackSrc && !failedFallback}
	<img class="art-image" src={fallbackSrc} {alt} loading="lazy" on:error={() => (failedFallback = true)} />
{:else}
	<div class:artist={kind === 'artist'} class="art-fallback" aria-label={alt}>
		{#if kind === 'artist'}
			<UserRound size={28} strokeWidth={1.5} />
		{:else}
			<Disc3 size={28} strokeWidth={1.5} />
		{/if}
		<span>{initials(alt)}</span>
	</div>
{/if}
