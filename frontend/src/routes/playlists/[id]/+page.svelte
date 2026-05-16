<script lang="ts">
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { api } from '$lib/api';
	import type { PlaylistDetail } from '$lib/types';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ImageWithFallback from '$lib/components/ImageWithFallback.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { coverUrl, formatDuration, formatNumber } from '$lib/format';
	import { player, playQueue, type QueueTrack } from '$lib/player';

	let detail: PlaylistDetail | null = null;
	let loading = true;
	let error = '';
	let loadedPlaylistId = '';

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

	function queue(): QueueTrack[] {
		const currentDetail = detail;
		if (!currentDetail) return [];
		return currentDetail.tracks.map((track) => ({
			id: track[0],
			title: track[1],
			artist: track[2] ?? 'Unknown artist',
			album: track[4] ?? currentDetail.playlist.name,
			albumId: track[3],
			coverArtId: track[5] ?? currentDetail.playlist.cover_art_id,
			duration: track[6]
		}));
	}

	function play(startIndex = 0) {
		playQueue(queue(), startIndex);
	}

	$: playlistId = $page.params.id;
	$: if (browser && playlistId && playlistId !== loadedPlaylistId) void load(playlistId);
	$: playlistCoverArtId = detail?.playlist.cover_art_id ?? detail?.tracks.find((track) => track[5])?.[5] ?? null;
	$: playingTrackId = $player.isPlaying ? $player.queue[$player.currentIndex]?.id ?? null : null;
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
			<button class="button" on:click={() => play(0)} disabled={!detail.tracks.length}>Play Playlist</button>
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
				</tr>
			</thead>
			<tbody>
				{#each detail.tracks as track, index}
					<tr class:playing-row={track[0] === playingTrackId} on:click={() => play(index)}>
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
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}
