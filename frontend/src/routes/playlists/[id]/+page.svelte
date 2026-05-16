<script lang="ts">
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { CheckCircle2, Download, Loader2 } from 'lucide-svelte';
	import { api } from '$lib/api';
	import type { PlaylistDetail } from '$lib/types';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ImageWithFallback from '$lib/components/ImageWithFallback.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { coverUrl, formatDuration, formatNumber } from '$lib/format';
	import { downloadPlaylist, downloadTrack, localMedia, type DownloadProgress } from '$lib/localMedia';
	import { player, playQueue, type QueueTrack } from '$lib/player';
	import { swipeQueue } from '$lib/swipeQueue';

	let detail: PlaylistDetail | null = null;
	let loading = true;
	let error = '';
	let loadedPlaylistId = '';
	let downloadingPlaylist = false;
	let downloadProgress: DownloadProgress | null = null;
	let downloadingTracks = new Set<number>();

	async function load(playlistId: string) {
		loading = true;
		error = '';
		detail = null;
		try {
			detail = await api.playlist(playlistId);
			loadedPlaylistId = playlistId;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load playlist';
		} finally {
			loading = false;
		}
	}

	function trackToQueueItem(track: PlaylistDetail['tracks'][number], currentDetail: PlaylistDetail): QueueTrack {
		return {
			id: track[0],
			title: track[1],
			artist: track[2] ?? 'Unknown artist',
			album: track[4] ?? currentDetail.playlist.name,
			albumId: track[3],
			coverArtId: track[5] ?? currentDetail.playlist.cover_art_id,
			duration: track[6]
		};
	}

	function queue(): QueueTrack[] {
		const currentDetail = detail;
		if (!currentDetail) return [];
		return currentDetail.tracks.map((track) => trackToQueueItem(track, currentDetail));
	}

	function play(startIndex = 0) {
		playQueue(queue(), startIndex);
	}

	async function savePlaylistOffline() {
		if (!detail || downloadingPlaylist) return;
		downloadingPlaylist = true;
		downloadProgress = null;
		try {
			await downloadPlaylist(detail, (progress) => {
				downloadProgress = progress;
			});
		} catch (error) {
			console.warn('Unable to download playlist', error);
		} finally {
			downloadingPlaylist = false;
			downloadProgress = null;
		}
	}

	async function saveTrackOffline(track: PlaylistDetail['tracks'][number], index: number) {
		if (!detail || downloadingTracks.has(track[0])) return;
		downloadingTracks = new Set([...downloadingTracks, track[0]]);
		try {
			await downloadTrack(playlistQueue[index], {
				playlist: {
					id: detail.playlist.id,
					name: detail.playlist.name,
					durationSeconds: detail.playlist.duration_seconds,
					coverArtId: detail.playlist.cover_art_id,
					sourceSongCount: detail.playlist.song_count,
					genre: track[7]
				},
				album: track[3]
					? {
							id: track[3],
							title: track[4] ?? detail.playlist.name,
							artistName: track[2] ?? 'Unknown artist',
							year: null,
							genre: track[7],
							coverArtId: track[5] ?? detail.playlist.cover_art_id,
							sourceTrackCount: null
						}
					: undefined
			});
		} catch (error) {
			console.warn('Unable to download playlist track', error);
		} finally {
			downloadingTracks.delete(track[0]);
			downloadingTracks = new Set(downloadingTracks);
		}
	}

	$: playlistId = $page.params.id;
	$: if (browser && playlistId && playlistId !== loadedPlaylistId) void load(playlistId);
	$: playlistCoverArtId = detail?.playlist.cover_art_id ?? detail?.tracks.find((track) => track[5])?.[5] ?? null;
	$: playlistQueue = queue();
	$: playingTrackId = $player.isPlaying ? $player.queue[$player.currentIndex]?.id ?? null : null;
	$: downloadedTrackIds = new Set(Object.keys($localMedia.tracks).map(Number));
	$: playlistDownloaded = (detail?.tracks.length ?? 0) > 0 && detail!.tracks.every((track) => downloadedTrackIds.has(track[0]));
</script>

{#if loading}
	<div class="skeleton-card"></div>
{:else if error}
	<ErrorState message={error} retry={() => { if (playlistId) void load(playlistId); }} />
{:else if !detail?.playlist}
	<EmptyState title="Playlist not found" />
{:else}
	<section class="detail-hero">
		<div class="detail-art">
			<ImageWithFallback src={coverUrl(playlistCoverArtId)} alt={detail.playlist.name} />
		</div>
		<div>
			<p class="eyebrow">Playlist</p>
			<h2>{detail.playlist.name}</h2>
			<div class="metric-grid compact">
				<StatCard label="Tracks" value={formatNumber(detail.tracks.length)} />
				<StatCard label="Duration" value={formatDuration(detail.playlist.duration_seconds)} />
				<StatCard label="Source" value="Navidrome" />
			</div>
			<div class="action-row">
				<button class="button" on:click={() => play(0)} disabled={!detail.tracks.length}>Play Playlist</button>
				<button class="button ghost" on:click={savePlaylistOffline} disabled={!detail.tracks.length || downloadingPlaylist || playlistDownloaded}>
					{#if downloadingPlaylist}<Loader2 size={16} />{:else if playlistDownloaded}<CheckCircle2 size={16} />{:else}<Download size={16} />{/if}
					{playlistDownloaded ? 'Downloaded' : downloadingPlaylist ? `${downloadProgress?.completed ?? 0}/${downloadProgress?.total ?? detail.tracks.length}` : 'Download Lossless'}
				</button>
			</div>
		</div>
	</section>

	<div class="table-wrap playlist-table-wrap">
		<table class="track-table playlist-track-table">
			<thead>
				<tr>
					<th></th>
					<th></th>
					<th>Track</th>
					<th>Artist</th>
					<th>Album</th>
					<th><span class="playlist-desktop-duration-heading">Duration</span><span class="playlist-mobile-duration-heading">Time</span></th>
					<th></th>
				</tr>
			</thead>
			<tbody>
				{#each detail.tracks as track, index}
					<tr use:swipeQueue={{ track: playlistQueue[index] }} class:playing-row={track[0] === playingTrackId} on:click={() => play(index)}>
						<td>
							{#if track[0] === playingTrackId}
								<div class="playing-indicator" aria-label="Now playing"><span></span><span></span><span></span></div>
							{:else}
								<button class="icon-button" aria-label={`Play ${track[1]}`} on:click|stopPropagation={() => play(index)}>▶</button>
							{/if}
						</td>
						<td>
							<div class="playlist-track-art">
								<ImageWithFallback src={coverUrl(track[5] ?? playlistCoverArtId)} alt={track[1]} />
							</div>
						</td>
						<td>
							<span class="playlist-track-title">{track[1]}</span>
							<span class="playlist-mobile-artist">{track[2] ?? 'Unknown artist'}</span>
						</td>
						<td>{track[2] ?? 'Unknown artist'}</td>
						<td>{track[4] ?? track[7] ?? '—'}</td>
						<td>{formatDuration(track[6])}</td>
						<td>
							<button
								class="icon-button"
								aria-label={downloadedTrackIds.has(track[0]) ? `${track[1]} downloaded` : `Download ${track[1]}`}
								disabled={downloadedTrackIds.has(track[0]) || downloadingTracks.has(track[0])}
								on:click|stopPropagation={() => saveTrackOffline(track, index)}
							>
								{#if downloadingTracks.has(track[0])}<Loader2 size={16} />{:else if downloadedTrackIds.has(track[0])}<CheckCircle2 size={16} />{:else}<Download size={16} />{/if}
							</button>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}
