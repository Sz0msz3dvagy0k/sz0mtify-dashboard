<script lang="ts">
	import { onMount } from 'svelte';
	import ImageWithFallback from './ImageWithFallback.svelte';
	import { api } from '$lib/api';
	import { coverUrl, formatNumber } from '$lib/format';

	export let id: number;
	export let name: string;
	export let albums = 0;
	export let tracks = 0;
	export let plays = 0;
	export let artistImageUrl: string | null = null;
	export let coverArtId: string | null = null;
	export let coverAlbumId: number | null = null;

	let resolvedCoverArtId: string | null = null;

	onMount(async () => {
		if (coverArtId || !coverAlbumId) return;
		try {
			const detail = await api.album(coverAlbumId);
			resolvedCoverArtId = detail.album?.[6] ?? null;
		} catch (error) {
			console.warn('Unable to load artist cover metadata', {
				artistId: id,
				albumId: coverAlbumId,
				error: error instanceof Error ? error.message : String(error)
			});
		}
	});

	$: fallbackImageSrc = coverUrl(coverArtId ?? resolvedCoverArtId);
</script>

<a class="media-card artist-card" href={`/artists/${id}`}>
	<div class="media-cover">
		<ImageWithFallback src={artistImageUrl} fallbackSrc={fallbackImageSrc} alt={name} kind="artist" />
	</div>
	<div class="media-body">
		<strong class="artist-name">{name}</strong>
		<span>{formatNumber(albums)} albums • {formatNumber(tracks)} tracks</span>
		<em>{formatNumber(plays)} plays</em>
	</div>
</a>
