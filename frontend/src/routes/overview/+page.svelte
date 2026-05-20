<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { formatBytes, formatNumber } from '$lib/format';
	import type { AlbumTuple, DiscoveryList, ListeningStats, MetadataHealth, Overview, StorageStats } from '$lib/types';
	import StatCard from '$lib/components/StatCard.svelte';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import AlbumCard from '$lib/components/AlbumCard.svelte';
	import DiscoveryCard from '$lib/components/DiscoveryCard.svelte';
	import TrackRow from '$lib/components/TrackRow.svelte';
	import SkeletonCard from '$lib/components/SkeletonCard.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import { trackHref } from '$lib/navigation';

	let loading = true;
	let error = '';
	let overview: Overview;
	let storage: StorageStats;
	let metadata: MetadataHealth;
	let listening: ListeningStats;
	let albums: AlbumTuple[] = [];
	let discovery: DiscoveryList | null = null;

	async function load() {
		loading = true;
		error = '';
		try {
			[overview, storage, metadata, listening, albums, discovery] = await Promise.all([
				api.overview(),
				api.storage(),
				api.metadataHealth(),
				api.listening(),
				api.albums(),
				api.newReleases('limit=4')
			]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load overview';
		} finally {
			loading = false;
		}
	}

	$: healthScore = overview ? Math.round(((overview.total_tracks - metadata.missing_genre) / Math.max(overview.total_tracks, 1)) * 100) : 0;
	$: genreData = storage?.size_by_genre.slice(0, 8).map(([name, bytes]) => ({ name: name ?? 'Unknown', value: bytes })) ?? [];

	onMount(load);

	function albumArtistName(album: AlbumTuple): string {
		return album[6] ?? 'Unknown artist';
	}
</script>

{#if loading}
	<div class="grid"><SkeletonCard /><SkeletonCard /><SkeletonCard /></div>
{:else if error}
	<ErrorState message={error} retry={load} />
{:else}
	<section class="metric-grid hero-grid">
		<StatCard label="Total Tracks" value={formatNumber(overview.total_tracks)} meta="indexed songs" />
		<StatCard label="Total Albums" value={formatNumber(overview.total_albums)} meta="library releases" />
		<StatCard label="Total Artists" value={formatNumber(overview.total_artists)} meta="credited names" />
		<StatCard label="Storage Used" value={formatBytes(storage.total_storage_bytes)} meta="track files only" />
	</section>

	<section class="dashboard-grid">
		<ChartCard
			title="Genre Weight"
			option={{
				series: [{ type: 'pie', radius: ['45%', '72%'], data: genreData, color: ['#f5f5f5', '#d4d4d4', '#a3a3a3', '#737373', '#525252', '#404040', '#333', '#262626'] }]
			}}
		/>
		<ChartCard
			title="Top Artists"
			option={{
				xAxis: { type: 'value', axisLabel: { color: '#8a8a8a' }, splitLine: { lineStyle: { color: '#262626' } } },
				yAxis: { type: 'category', data: listening.top_artists.slice(0, 8).map((row) => row[1] ?? 'Unknown').reverse(), axisLabel: { color: '#a3a3a3' } },
				series: [{ type: 'bar', data: listening.top_artists.slice(0, 8).map((row) => row[2]).reverse(), color: '#f5f5f5' }]
			}}
		/>
	</section>

	<section class="split-grid">
		<div>
			<SectionHeader title="Recently Indexed Albums" eyebrow={`${albums.length} shown`} action="All albums" href="/albums" />
			<div class="album-strip">
				{#each albums.slice(0, 6) as album}
					<AlbumCard id={album[0]} title={album[1]} artist={albumArtistName(album)} year={album[3]} genre={album[4]} coverArtId={album[5]} />
				{/each}
			</div>
		</div>
		<div>
			<SectionHeader title="Listening Profile" eyebrow={listening.data_source.replaceAll('_', ' ')} action="Listening" href="/listening" />
			<div class="panel-list">
				{#each listening.top_tracks.slice(0, 6) as track}
					<TrackRow title={track[1]} count={track[3]} href={trackHref(track[0])} />
				{/each}
			</div>
		</div>
	</section>

	<section class="split-grid">
		<div class="health-panel">
			<SectionHeader title="Metadata Health" eyebrow={`${healthScore}% score`} action="Diagnostics" href="/metadata-health" />
			<div class="progress"><span style={`width:${healthScore}%`}></span></div>
			<div class="mini-stats">
				<span>Missing genre <strong>{formatNumber(metadata.missing_genre)}</strong></span>
				<span>Missing MBID <strong>{formatNumber(metadata.missing_mbid)}</strong></span>
			</div>
		</div>
		<div>
			<SectionHeader title="Discovery Radar" eyebrow={`${discovery?.total ?? 0} candidates`} action="Discovery" href="/discovery" />
			<div class="compact-cards">
				{#each discovery?.items.slice(0, 2) ?? [] as item}
					<DiscoveryCard {item} />
				{/each}
			</div>
		</div>
	</section>
{/if}
