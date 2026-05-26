<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { AlbumTuple, ArtistTuple, DiscoveryList, StorageStats } from '$lib/types';
	import ArtistCard from '$lib/components/ArtistCard.svelte';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ExpandableTable from '$lib/components/ExpandableTable.svelte';
	import FilterBar from '$lib/components/FilterBar.svelte';
	import ItemsPerPage from '$lib/components/ItemsPerPage.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import SkeletonCard from '$lib/components/SkeletonCard.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { formatBytes, formatNumber } from '$lib/format';

	let artists: ArtistTuple[] = [];
	let albums: AlbumTuple[] = [];
	let storage: StorageStats | null = null;
	let similar: DiscoveryList | null = null;
	let loading = true;
	let error = '';
	let filter = '';
	let itemsPerPage = 18;
	let page = 1;

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
	$: representativeAlbumByArtist = new Map(
		albums.filter((album) => album[2] !== null).map((album) => [album[2] as number, album[0]])
	);
	$: storageArtistRows = storage?.size_by_artist.slice(0, 10) ?? [];
	$: storageArtistChartRows = [...storageArtistRows].reverse();
	$: storageArtistTableRows = storageArtistRows.map((row, index) => ({
		id: row[0] ?? `artist-${index}`,
		title: row[1] ?? 'Unknown artist',
		titleClass: 'artist-name',
		href: row[0] ? `/artists/${row[0]}` : null,
		details: [
			['Storage', formatBytes(row[2])],
			['Tracks', row[3]]
		] as [string, string | number | null | undefined][]
	}));
	$: pageStart = (page - 1) * itemsPerPage;
	$: visibleArtists = filtered.slice(pageStart, pageStart + itemsPerPage);
	$: if (page > Math.max(1, Math.ceil(filtered.length / itemsPerPage))) page = 1;

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
			title="Storage by Artist"
			option={{
				grid: { left: 8, right: 28, top: 18, bottom: 28 },
				xAxis: { type: 'value', axisLabel: { color: '#8a8a8a', formatter: (value: number) => `${value} MB` }, splitLine: { lineStyle: { color: '#262626' } } },
				yAxis: { type: 'category', data: storageArtistChartRows.map((row) => row[1] ?? 'Unknown'), axisLabel: { color: '#a3a3a3' } },
				series: [{ type: 'bar', data: storageArtistChartRows.map((row) => Number((row[2] / 1024 ** 2).toFixed(1))), color: '#f5f5f5' }],
				tooltip: { valueFormatter: (value: number) => `${value.toFixed(1)} MB` }
			}}
		/>
		<ExpandableTable title="Artist Storage" rows={storageArtistTableRows} />
	</section>

	<div class="toolbar"><FilterBar bind:value={filter} placeholder="Filter artists" /></div>
	<SectionHeader title="Artist Grid" eyebrow={`${filtered.length} matches`} />
	<div class="media-grid artist-grid">
		{#each visibleArtists as artist}
			<ArtistCard id={artist[0]} name={artist[1]} albums={artist[2] ?? 0} tracks={artist[3] ?? 0} plays={artist[4] ?? 0} artistImageUrl={artist[5]} coverArtId={artist[6]} coverAlbumId={representativeAlbumByArtist.get(artist[0]) ?? null} />
		{/each}
	</div>
	<ItemsPerPage bind:value={itemsPerPage} bind:page total={filtered.length} shown={visibleArtists.length} />
{/if}
