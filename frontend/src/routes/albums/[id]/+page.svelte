<script lang="ts">
	import { browser } from '$app/environment';
	import { page } from '$app/stores';
	import { onDestroy } from 'svelte';
	import { api } from '$lib/api';
	import type { AlbumDetail, ArtistTuple } from '$lib/types';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ImageWithFallback from '$lib/components/ImageWithFallback.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { coverUrl, formatNumber } from '$lib/format';
	import { player, playQueue, type QueueTrack } from '$lib/player';

	let detail: AlbumDetail | null = null;
	let artists: ArtistTuple[] = [];
	let error = '';
	let loading = true;
	let loadedAlbumId: number | null = null;
	let highlightedTrackId: number | null = null;
	let lastHighlightedTrackParam: number | null = null;
	let highlightTimer: ReturnType<typeof setTimeout> | null = null;

	async function load(albumId: number) {
		loading = true;
		error = '';
		detail = null;
		try {
			[detail, artists] = await Promise.all([api.album(albumId), api.artists()]);
			loadedAlbumId = albumId;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load album';
		} finally {
			loading = false;
		}
	}
	onDestroy(() => {
		if (highlightTimer) clearTimeout(highlightTimer);
	});
	$: album = detail?.album;
	$: albumId = Number($page.params.id);
	$: if (browser && Number.isFinite(albumId) && albumId > 0 && albumId !== loadedAlbumId) void load(albumId);
	$: artistName = detail?.artist_name ?? artists.find((artist) => artist[0] === album?.[2])?.[1] ?? 'Unknown artist';
	$: albumTracks = detail?.tracks ?? [];
	$: playingTrackId = $player.isPlaying ? $player.queue[$player.currentIndex]?.id ?? null : null;
	$: trackParam = Number($page.url.searchParams.get('track'));
	$: if (!loading && Number.isFinite(trackParam) && trackParam > 0 && trackParam !== lastHighlightedTrackParam) {
		lastHighlightedTrackParam = trackParam;
		highlightedTrackId = trackParam;
		if (highlightTimer) clearTimeout(highlightTimer);
		highlightTimer = setTimeout(() => {
			highlightedTrackId = null;
			highlightTimer = null;
		}, 1200);
	}

	function queue(): QueueTrack[] {
		if (!album) return [];
		return albumTracks.map((track) => ({
			id: track[0],
			title: track[1],
			artist: artistName,
			album: album[1],
			albumId: album[0],
			coverArtId: album[6],
			duration: null
		}));
	}

	function play(startIndex = 0) {
		playQueue(queue(), startIndex);
	}
</script>

{#if loading}
	<div class="skeleton-card"></div>
{:else if error}
	<ErrorState message={error} retry={() => { if (Number.isFinite(albumId) && albumId > 0) void load(albumId); }} />
{:else if !album}
	<EmptyState title="Album not found" />
{:else}
	<section class="detail-hero">
		<div class="detail-art">
			<ImageWithFallback src={coverUrl(album[6])} alt={album[1]} />
		</div>
		<div>
			<p class="eyebrow">{artistName}</p>
			<h2>{album[1]}</h2>
			<div class="metric-grid compact">
				<StatCard label="Year" value={album[3] ?? '—'} />
				<StatCard label="Genre" value={album[4] ?? '—'} />
				<StatCard label="Tracks" value={formatNumber(albumTracks.length)} />
			</div>
			<button class="button" on:click={() => play(0)} disabled={!albumTracks.length}>Play Album</button>
		</div>
	</section>
	<div class="table-wrap">
		<table class="track-table">
			<thead>
				<tr><th></th><th>#</th><th>Track</th><th>Disc</th></tr>
			</thead>
			<tbody>
				{#each albumTracks as track, index}
					<tr class:highlight-row={track[0] === highlightedTrackId} class:playing-row={track[0] === playingTrackId} on:click={() => play(index)}>
						<td>
							{#if track[0] === playingTrackId}
								<div class="playing-indicator" aria-label="Now playing"><span></span><span></span><span></span></div>
							{:else}
								<button class="icon-button" aria-label={`Play ${track[1]}`} on:click|stopPropagation={() => play(index)}>▶</button>
							{/if}
						</td>
						<td>{track[2] ?? '—'}</td>
						<td>{track[1]}</td>
						<td>{track[3] ?? 1}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}
