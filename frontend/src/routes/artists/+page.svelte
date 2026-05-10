<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { AlbumTuple, ArtistTuple, DiscoveryList, StorageStats } from '$lib/types';
	import ArtistCard from '$lib/components/ArtistCard.svelte';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import FilterBar from '$lib/components/FilterBar.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import SkeletonCard from '$lib/components/SkeletonCard.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { formatBytes, formatNumber } from '$lib/format';

	let artists: ArtistTuple[] = [];
	let albums: AlbumTuple[] = [];
	let storage: StorageStats;
	let similar: DiscoveryList | null = null;
	let loading = true;
	let error = '';
	let filter = '';

	async function load() {
		loading = true;
		error = '';
		try {
			[artists, albums, storage, similar] = await Promise.all([api.artists(), api.albums(), api.storage(), api.similarArtists('limit=6')]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load artists';
		} finally {
			loading = false;
		}
	}

	$: filtered = artists.filter((artist) => artist[1].toLowerCase().includes(filter.toLowerCase()));
	$: topByTracks = [...artists].sort((a, b) => (b[3] ?? 0) - (a[3] ?? 0)).slice(0, 10);
	$: representativeAlbumByArtist = new Map(
		albums.filter((album) => album[2] !== null).map((album) => [album[2] as number, album[0]])
	);

	onMount(load);
</script>

{#if loading}
	<div class="grid"><SkeletonCard /><SkeletonCard /><SkeletonCard /></div>
{:else if error}
	<ErrorState message={error} retry={load} />
{:else}
	<section class="metric-grid">
		<StatCard label="Artists" value={formatNumber(artists.length)} meta="indexed credits" />
		<StatCard label="One-album Artists" value={formatNumber(artists.filter((a) => (a[2] ?? 0) === 1).length)} meta="focused catalogs" />
		<StatCard label="Similar Leads" value={formatNumber(similar?.total ?? 0)} meta="from discovery" />
	</section>

	<section class="dashboard-grid">
		<ChartCard
			title="Track Depth"
			option={{
				xAxis: { type: 'value', axisLabel: { color: '#8a8a8a' }, splitLine: { lineStyle: { color: '#262626' } } },
				yAxis: { type: 'category', data: topByTracks.map((a) => a[1]).reverse(), axisLabel: { color: '#a3a3a3' } },
				series: [{ type: 'bar', data: topByTracks.map((a) => a[3] ?? 0).reverse(), color: '#f5f5f5' }]
			}}
		/>
		<DataTable columns={['Artist', 'Storage', 'Tracks']} rows={storage.size_by_artist.slice(0, 10).map((row) => [row[1], formatBytes(row[2]), row[3]])} />
	</section>

	<div class="toolbar"><FilterBar bind:value={filter} placeholder="Filter artists" /></div>
	<SectionHeader title="Artist Grid" eyebrow={`${filtered.length} matches`} />
	<div class="media-grid artist-grid">
		{#each filtered as artist}
			<ArtistCard id={artist[0]} name={artist[1]} albums={artist[2] ?? 0} tracks={artist[3] ?? 0} plays={artist[4] ?? 0} artistImageUrl={artist[5]} coverArtId={artist[6]} coverAlbumId={representativeAlbumByArtist.get(artist[0]) ?? null} />
		{/each}
	</div>
{/if}
