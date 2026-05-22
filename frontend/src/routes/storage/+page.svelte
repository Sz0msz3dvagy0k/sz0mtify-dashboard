<script lang="ts">
	import { onMount } from 'svelte';
	import { Trash2 } from 'lucide-svelte';
	import { api } from '$lib/api';
	import type { StorageStats } from '$lib/types';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ExpandableTable from '$lib/components/ExpandableTable.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { formatBytes } from '$lib/format';
	import {
		deleteLocalAlbum,
		deleteLocalPlaylist,
		deleteLocalTrack,
		loadLocalMedia,
		localMedia,
		localMediaStorageBudget,
		localMediaTotals,
		pruneLocalMediaToBudget,
		setLocalMediaStorageBudget
	} from '$lib/localMedia';

	let storage: StorageStats;
	let loading = true;
	let error = '';
	let localBusy = '';
	let localBudgetBytes = 0;
	let localBudgetGb = '8';
	async function load() {
		loading = true;
		try {
			const [, budget] = await Promise.all([loadLocalMedia(), localMediaStorageBudget()]);
			storage = await api.storage();
			localBudgetBytes = budget;
			localBudgetGb = (budget / 1024 ** 3).toFixed(1).replace(/\.0$/, '');
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load storage';
		} finally {
			loading = false;
		}
	}
	onMount(load);
	$: rankedArtists = storage?.size_by_artist.slice(0, 16).reverse() ?? [];
	$: rankedExtensions = storage?.extension_breakdown.slice(0, 12).reverse() ?? [];
	$: largestAlbumRows = storage?.largest_albums.slice(0, 12).map((album, index) => ({
		id: album[0] ?? `album-${index}`,
		title: album[1] ?? 'Unknown album',
		href: album[0] ? `/albums/${album[0]}` : null,
		details: [
			['Size', formatBytes(album[3])],
			['Tracks', album[4]]
		] as [string, string | number | null | undefined][]
	})) ?? [];
	$: largestTrackRows = storage?.largest_tracks.slice(0, 12).map((track) => ({
		id: track[0],
		title: track[1],
		href: track[3] ? `/albums/${track[3]}?track=${track[0]}` : null,
		details: [
			['Size', formatBytes(track[4])],
			['Format', track[6] ?? track[7] ?? '—']
		] as [string, string | number | null | undefined][]
	})) ?? [];
	$: suspiciousTrackRows = storage?.suspicious_large_tracks.map((track) => ({
		id: track[0],
		title: track[1],
		href: track[3] ? `/albums/${track[3]}?track=${track[0]}` : null,
		details: [
			['Size', formatBytes(track[2])],
			['Format', track[4] ?? '—']
		] as [string, string | number | null | undefined][]
	})) ?? [];
	$: localTotals = localMediaTotals($localMedia);
	$: localBudgetPercent = localBudgetBytes > 0 ? Math.min(100, (localTotals.bytes / localBudgetBytes) * 100) : 0;
	$: localTracks = Object.values($localMedia.tracks).sort((a, b) => b.downloadedAt.localeCompare(a.downloadedAt));
	$: localAlbums = Object.values($localMedia.albums).sort((a, b) => b.downloadedAt.localeCompare(a.downloadedAt));
	$: localPlaylists = Object.values($localMedia.playlists).sort((a, b) => b.downloadedAt.localeCompare(a.downloadedAt));

	async function runLocalAction(key: string, action: () => Promise<void>) {
		localBusy = key;
		try {
			await action();
			storage = await api.storage();
		} catch (error) {
			console.warn('Unable to update local media', error);
		} finally {
			localBusy = '';
		}
	}

	async function saveLocalBudget() {
		const gb = Number(localBudgetGb);
		if (!Number.isFinite(gb) || gb <= 0) return;
		await runLocalAction('budget', async () => {
			await setLocalMediaStorageBudget(gb * 1024 ** 3);
			localBudgetBytes = await localMediaStorageBudget();
			localBudgetGb = (localBudgetBytes / 1024 ** 3).toFixed(1).replace(/\.0$/, '');
		});
	}
</script>

{#if loading}
	<div class="skeleton-card"></div>
{:else if error}
	<ErrorState message={error} retry={load} />
{:else}
	<section class="metric-grid hero-grid">
		<StatCard label="Total Storage" value={formatBytes(storage.total_storage_bytes)} meta="SUM tracks.size_bytes" />
		<StatCard label="Track Bytes" value={formatBytes(storage.tracks_size_bytes)} meta="same source of truth" />
		<StatCard label="Average Track" value={formatBytes(storage.average_track_size_bytes)} />
		<StatCard label="MB / Minute" value={(storage.average_mb_per_minute ?? 0).toFixed(2)} />
	</section>
	<section class="local-media-section">
		<div class="section-heading">
			<div>
				<p class="eyebrow">Mobile downloads</p>
				<h2>Local Media</h2>
			</div>
			<span class="muted">{formatBytes(localTotals.bytes)} / {formatBytes(localBudgetBytes)} · {localTotals.tracks} tracks</span>
		</div>
		<section class="metric-grid compact">
			<StatCard label="Downloaded Tracks" value={localTotals.tracks} />
			<StatCard label="Downloaded Albums" value={localTotals.albums} />
			<StatCard label="Downloaded Playlists" value={localTotals.playlists} />
		</section>
		<div class="table-wrap local-media-panel">
			<header>Download Budget</header>
			<div class="local-budget-row">
				<div class="local-budget-meter" aria-label="Local media storage budget">
					<span style={`width: ${localBudgetPercent}%`}></span>
				</div>
				<label>
					<span>GB</span>
					<input type="number" min="0.125" step="0.5" bind:value={localBudgetGb} disabled={localBusy === 'budget'} />
				</label>
				<button class="button ghost" disabled={localBusy === 'budget'} on:click={saveLocalBudget}>Save Budget</button>
				<button class="button ghost" disabled={localBusy === 'prune'} on:click={() => runLocalAction('prune', pruneLocalMediaToBudget)}>Free Space</button>
			</div>
		</div>
		{#if localTotals.tracks}
			<div class="dashboard-grid">
				<div class="table-wrap local-media-panel">
					<header>Albums</header>
					{#if localAlbums.length}
						<div class="panel-list">
							{#each localAlbums as album}
								<div class="local-media-row">
									<a href={`/albums/${album.id}`}>
										<strong>{album.title}</strong>
										<span><span class="artist-name artist-meta">{album.artistName}</span> · {album.trackIds.length}/{album.sourceTrackCount ?? album.trackIds.length} tracks</span>
									</a>
									<button class="icon-button" aria-label={`Delete ${album.title}`} disabled={localBusy === `album-${album.id}`} on:click={() => runLocalAction(`album-${album.id}`, () => deleteLocalAlbum(album.id))}>
										<Trash2 size={16} />
									</button>
								</div>
							{/each}
						</div>
					{:else}
						<EmptyState title="No downloaded albums" />
					{/if}
				</div>
				<div class="table-wrap local-media-panel">
					<header>Playlists</header>
					{#if localPlaylists.length}
						<div class="panel-list">
							{#each localPlaylists as playlist}
								<div class="local-media-row">
									<a href={`/playlists/${encodeURIComponent(playlist.id)}`}>
										<strong>{playlist.name}</strong>
										<span>{playlist.trackIds.length}/{playlist.sourceSongCount ?? playlist.trackIds.length} tracks</span>
									</a>
									<button class="icon-button" aria-label={`Delete ${playlist.name}`} disabled={localBusy === `playlist-${playlist.id}`} on:click={() => runLocalAction(`playlist-${playlist.id}`, () => deleteLocalPlaylist(playlist.id))}>
										<Trash2 size={16} />
									</button>
								</div>
							{/each}
						</div>
					{:else}
						<EmptyState title="No downloaded playlists" />
					{/if}
				</div>
			</div>
			<div class="table-wrap local-media-panel">
				<header>Songs</header>
				<div class="panel-list">
					{#each localTracks as track}
						<div class="local-media-row">
							<a href={track.albumId ? `/albums/${track.albumId}?track=${track.id}` : undefined}>
								<strong>{track.title}</strong>
								<span><span class="artist-name artist-meta">{track.artist}</span> · {track.album} · {formatBytes(track.sizeBytes)}</span>
							</a>
							<button class="icon-button" aria-label={`Delete ${track.title}`} disabled={localBusy === `track-${track.id}`} on:click={() => runLocalAction(`track-${track.id}`, () => deleteLocalTrack(track.id))}>
								<Trash2 size={16} />
							</button>
						</div>
					{/each}
				</div>
			</div>
		{:else}
			<EmptyState title="No local media yet" detail="Download songs, albums, or playlists from their detail pages." />
		{/if}
	</section>
	<section class="dashboard-grid">
		<ChartCard
			title="Storage by Artist"
			option={{
				grid: { left: 132, right: 28, top: 18, bottom: 28 },
				xAxis: { type: 'value', axisLabel: { color: '#8a8a8a', formatter: (value: number) => `${value} MB` }, splitLine: { lineStyle: { color: '#262626' } } },
				yAxis: { type: 'category', data: rankedArtists.map(([, name]) => name ?? 'Unknown'), axisLabel: { color: '#a3a3a3' } },
				series: [{ type: 'bar', data: rankedArtists.map(([, , bytes]) => Number((bytes / 1024 ** 2).toFixed(1))), color: '#f5f5f5' }],
				tooltip: { valueFormatter: (value: number) => `${value.toFixed(1)} MB` }
			}}
			height={360}
		/>
		<ChartCard
			title="Extension Breakdown"
			option={{
				grid: { left: 80, right: 28, top: 18, bottom: 28 },
				xAxis: { type: 'value', axisLabel: { color: '#8a8a8a' }, splitLine: { lineStyle: { color: '#262626' } } },
				yAxis: { type: 'category', data: rankedExtensions.map(([name]) => name ?? 'unknown'), axisLabel: { color: '#a3a3a3' } },
				series: [{ type: 'bar', data: rankedExtensions.map(([, tracks]) => tracks), color: '#d4d4d4' }]
			}}
		/>
	</section>
	<ExpandableTable title="Largest Albums" rows={largestAlbumRows} />
	<ExpandableTable title="Largest Tracks" rows={largestTrackRows} />
	{#if storage.suspicious_large_tracks.length}
		<ExpandableTable title="Suspicious Tracks" rows={suspiciousTrackRows} />
	{:else}
		<EmptyState title="No suspicious large files" />
	{/if}
{/if}
