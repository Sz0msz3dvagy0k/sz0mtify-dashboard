<script lang="ts">
	import { browser } from '$app/environment';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ImageWithFallback from '$lib/components/ImageWithFallback.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import TrackActionsMenu from '$lib/components/TrackActionsMenu.svelte';
	import { coverUrl, formatDuration, formatNumber } from '$lib/format';
	import { downloadTrack, localMedia } from '$lib/localMedia';
	import { playQueue, type QueueTrack } from '$lib/player';
	import type { TrackDetail } from '$lib/types';
	import { CheckCircle2, Download, Loader2, Play } from 'lucide-svelte';

	let detail: TrackDetail | null = null;
	let loading = true;
	let error = '';
	let loadedTrackId: number | null = null;
	let downloading = false;

	async function load(trackId: number) {
		loading = true;
		error = '';
		detail = null;
		try {
			detail = await api.track(trackId);
			loadedTrackId = trackId;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load track';
		} finally {
			loading = false;
		}
	}

	function queueTrack(): QueueTrack | null {
		if (!track) return null;
		return {
			id: track[0],
			title: track[1],
			artist: track[3] ?? 'Unknown artist',
			album: track[5] ?? 'Unknown album',
			albumId: track[4],
			coverArtId: track[6],
			duration: track[7]
		};
	}

	function play() {
		const item = queueTrack();
		if (!item) return;
		playQueue([item], 0);
	}

	async function saveTrackOffline() {
		const item = queueTrack();
		if (!item || downloading) return;
		downloading = true;
		try {
			await downloadTrack(item, {
				album: item.albumId
					? {
							id: item.albumId,
							title: item.album,
							artistName: item.artist,
							year: track?.[10] ?? null,
							genre: track?.[11] ?? null,
							coverArtId: item.coverArtId,
							sourceTrackCount: null,
							trackNumber: track?.[8] ?? null,
							discNumber: track?.[9] ?? null
						}
					: undefined
			});
		} catch (error) {
			console.warn('Unable to download track', error);
		} finally {
			downloading = false;
		}
	}

	$: trackId = Number($page.params.id);
	$: if (browser && Number.isFinite(trackId) && trackId > 0 && trackId !== loadedTrackId) void load(trackId);
	$: track = detail?.track;
	$: downloadedTrackIds = new Set(Object.keys($localMedia.tracks).map(Number));
	$: downloaded = track ? downloadedTrackIds.has(track[0]) : false;
	$: queueItem = queueTrack();
</script>

{#if loading}
	<div class="skeleton-card"></div>
{:else if error}
	<ErrorState message={error} retry={() => { if (Number.isFinite(trackId) && trackId > 0) void load(trackId); }} />
{:else if !track}
	<EmptyState title="Track not found" />
{:else}
	<section class="detail-hero track-detail-hero">
		<div class="detail-art">
			<ImageWithFallback src={coverUrl(track[6])} alt={track[1]} />
		</div>
		<div>
			<p class="eyebrow">Track</p>
			<h2>{track[1]}</h2>
			<div class="track-meta-links">
				{#if track[2]}
					<a href={`/artists/${track[2]}`}>{track[3] ?? 'Unknown artist'}</a>
				{:else}
					<span>{track[3] ?? 'Unknown artist'}</span>
				{/if}
				<span>•</span>
				{#if track[4]}
					<a href={`/albums/${track[4]}`}>{track[5] ?? 'Unknown album'}</a>
				{:else}
					<span>{track[5] ?? 'Unknown album'}</span>
				{/if}
			</div>
			<div class="metric-grid compact">
				<StatCard label="Duration" value={formatDuration(track[7])} />
				<StatCard label="Plays" value={formatNumber(track[12] ?? 0)} />
				<StatCard label="Year" value={track[10] ?? '—'} />
			</div>
			<div class="action-row">
				<button class="button" on:click={play}><Play size={16} />Play Track</button>
				<button class="button ghost" on:click={saveTrackOffline} disabled={downloaded || downloading}>
					{#if downloading}<Loader2 size={16} />{:else if downloaded}<CheckCircle2 size={16} />{:else}<Download size={16} />{/if}
					{downloaded ? 'Downloaded' : 'Download Lossless'}
				</button>
				{#if queueItem}
					<TrackActionsMenu
						track={queueItem}
						artistHref={track[2] ? `/artists/${track[2]}` : null}
						onDownload={saveTrackOffline}
						{downloaded}
						{downloading}
					/>
				{/if}
			</div>
		</div>
	</section>
{/if}
