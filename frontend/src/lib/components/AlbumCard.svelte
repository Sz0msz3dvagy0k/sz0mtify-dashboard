<script lang="ts">
	import { onMount } from 'svelte';
	import ImageWithFallback from './ImageWithFallback.svelte';
	import { api } from '$lib/api';
	import { coverUrl } from '$lib/format';

	export let id: number;
	export let title: string;
	export let artist = 'Unknown artist';
	export let year: number | null = null;
	export let genre: string | null = null;
	export let coverArtId: string | null = null;

	let resolvedCoverArtId: string | null = null;

	onMount(async () => {
		if (coverArtId) return;
		try {
			const detail = await api.album(id);
			resolvedCoverArtId = detail.album?.[6] ?? null;
		} catch (error) {
			console.warn('Unable to load album cover metadata', {
				albumId: id,
				error: error instanceof Error ? error.message : String(error)
			});
		}
	});

	$: imageUrl = coverUrl(coverArtId ?? resolvedCoverArtId);
</script>

<a class="media-card" href={`/albums/${id}`}>
	<div class="media-cover">
		<ImageWithFallback src={imageUrl} alt={title} />
	</div>
	<div class="media-body">
		<strong>{title}</strong>
		<span><span class="artist-name artist-meta">{artist}</span>{year ? ` • ${year}` : ''}</span>
		{#if genre}<em>{genre}</em>{/if}
	</div>
</a>
