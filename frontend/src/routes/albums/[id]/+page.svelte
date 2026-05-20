<script lang="ts">
	import { browser } from '$app/environment';
	import { page } from '$app/stores';
	import { onDestroy } from 'svelte';
	import { CheckCircle2, Download, Loader2 } from 'lucide-svelte';
	import { api } from '$lib/api';
	import type { AlbumDetail, ArtistTuple } from '$lib/types';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ImageWithFallback from '$lib/components/ImageWithFallback.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import TrackActionsMenu from '$lib/components/TrackActionsMenu.svelte';
	import { coverUrl, formatDuration, formatNumber } from '$lib/format';
	import { downloadAlbum, downloadTrack, localMedia, type DownloadProgress } from '$lib/localMedia';
	import { player, playQueue, type QueueTrack } from '$lib/player';
	import { swipeQueue } from '$lib/swipeQueue';

	let detail: AlbumDetail | null = null;
	let artists: ArtistTuple[] = [];
	let error = '';
	let loading = true;
	let loadedAlbumId: number | null = null;
	let highlightedTrackId: number | null = null;
	let lastHighlightedTrackParam: number | null = null;
	let highlightTimer: ReturnType<typeof setTimeout> | null = null;
	let downloadingAlbum = false;
	let downloadProgress: DownloadProgress | null = null;
	let downloadingTracks = new Set<number>();

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
	$: albumQueue = album ? albumTracks.map(trackToQueueItem) : [];
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

	function trackToQueueItem(track: AlbumDetail['tracks'][number]): QueueTrack {
		return {
			id: track[0],
			title: track[1],
			artist: artistName,
			album: album?.[1] ?? 'Unknown album',
			albumId: album?.[0] ?? null,
			coverArtId: album?.[6] ?? null,
			duration: track[4]
		};
	}

	function queue(): QueueTrack[] {
		if (!album) return [];
		return albumQueue;
	}

	function play(startIndex = 0) {
		playQueue(queue(), startIndex);
	}

	async function saveAlbumOffline() {
		if (!detail?.album || downloadingAlbum) return;
		downloadingAlbum = true;
		downloadProgress = null;
		try {
			await downloadAlbum(detail, artistName, (progress) => {
				downloadProgress = progress;
			});
		} catch (error) {
			console.warn('Unable to download album', error);
		} finally {
			downloadingAlbum = false;
			downloadProgress = null;
		}
	}

	async function saveTrackOffline(track: AlbumDetail['tracks'][number], index: number) {
		if (!album || downloadingTracks.has(track[0])) return;
		downloadingTracks = new Set([...downloadingTracks, track[0]]);
		try {
			await downloadTrack(albumQueue[index], {
				album: {
					id: album[0],
					title: album[1],
					artistName,
					year: album[3],
					genre: album[4],
					coverArtId: album[6],
					sourceTrackCount: album[5],
					trackNumber: track[2],
					discNumber: track[3]
				}
			});
		} catch (error) {
			console.warn('Unable to download track', error);
		} finally {
			downloadingTracks.delete(track[0]);
			downloadingTracks = new Set(downloadingTracks);
		}
	}

	$: downloadedTrackIds = new Set(Object.keys($localMedia.tracks).map(Number));
	$: albumDownloaded = albumTracks.length > 0 && albumTracks.every((track) => downloadedTrackIds.has(track[0]));
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
			<div class="action-row">
				<button class="button" on:click={() => play(0)} disabled={!albumTracks.length}>Play Album</button>
				<button class="button ghost" on:click={saveAlbumOffline} disabled={!albumTracks.length || downloadingAlbum || albumDownloaded}>
					{#if downloadingAlbum}<Loader2 size={16} />{:else if albumDownloaded}<CheckCircle2 size={16} />{:else}<Download size={16} />{/if}
					{albumDownloaded ? 'Downloaded' : downloadingAlbum ? `${downloadProgress?.completed ?? 0}/${downloadProgress?.total ?? albumTracks.length}` : 'Download Lossless'}
				</button>
			</div>
		</div>
	</section>
	<div class="table-wrap">
		<table class="track-table">
			<thead>
				<tr><th></th><th>#</th><th>Track</th><th>Time</th><th></th></tr>
			</thead>
			<tbody>
				{#each albumTracks as track, index}
					<tr use:swipeQueue={{ track: albumQueue[index] }} class:highlight-row={track[0] === highlightedTrackId} class:playing-row={track[0] === playingTrackId} on:click={() => play(index)}>
						<td>
							{#if track[0] === playingTrackId}
								<div class="playing-indicator" aria-label="Now playing"><span></span><span></span><span></span></div>
							{:else}
								<button class="icon-button" aria-label={`Play ${track[1]}`} on:click|stopPropagation={() => play(index)}>▶</button>
							{/if}
						</td>
						<td>{track[2] ?? '—'}</td>
						<td>{track[1]}</td>
						<td>{formatDuration(track[4])}</td>
						<td>
							<TrackActionsMenu
								track={albumQueue[index]}
								artistHref={album?.[2] ? `/artists/${album[2]}` : null}
								onDownload={() => saveTrackOffline(track, index)}
								downloaded={downloadedTrackIds.has(track[0])}
								downloading={downloadingTracks.has(track[0])}
							/>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}
