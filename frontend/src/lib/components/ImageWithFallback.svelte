<script lang="ts">
	import { Disc3, UserRound } from 'lucide-svelte';
	import { initials } from '$lib/format';
	import { loadCachedImage } from '$lib/imageCache';

	export let src: string | null = null;
	export let fallbackSrc: string | null = null;
	export let alt = '';
	export let kind: 'album' | 'artist' = 'album';
	let failedPrimary = false;
	let failedFallback = false;
	let lastSrc: string | null = null;
	let lastFallbackSrc: string | null = null;
	let primaryDisplaySrc: string | null = null;
	let fallbackDisplaySrc: string | null = null;
	let primaryRequestId = 0;
	let fallbackRequestId = 0;

	$: if (src !== lastSrc) {
		lastSrc = src;
		failedPrimary = false;
		primaryDisplaySrc = null;
		if (src) void resolvePrimary(src, ++primaryRequestId);
	}
	$: wantedFallbackSrc = !src || failedPrimary ? fallbackSrc : null;
	$: if (wantedFallbackSrc !== lastFallbackSrc) {
		lastFallbackSrc = wantedFallbackSrc;
		failedFallback = false;
		fallbackDisplaySrc = null;
		if (wantedFallbackSrc) void resolveFallback(wantedFallbackSrc, ++fallbackRequestId);
	}

	async function resolvePrimary(value: string, requestId: number) {
		try {
			const cached = await loadCachedImage(value);
			if (requestId === primaryRequestId && value === src) primaryDisplaySrc = cached;
		} catch {
			if (requestId === primaryRequestId && value === src) failedPrimary = true;
		}
	}

	async function resolveFallback(value: string, requestId: number) {
		try {
			const cached = await loadCachedImage(value);
			if (requestId === fallbackRequestId && value === wantedFallbackSrc) fallbackDisplaySrc = cached;
		} catch {
			if (requestId === fallbackRequestId && value === wantedFallbackSrc) failedFallback = true;
		}
	}
</script>

{#if src && !failedPrimary && primaryDisplaySrc}
	<img class="art-image" src={primaryDisplaySrc} {alt} loading="lazy" draggable="false" on:error={() => (failedPrimary = true)} />
{:else if wantedFallbackSrc && !failedFallback && fallbackDisplaySrc}
	<img class="art-image" src={fallbackDisplaySrc} {alt} loading="lazy" draggable="false" on:error={() => (failedFallback = true)} />
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
