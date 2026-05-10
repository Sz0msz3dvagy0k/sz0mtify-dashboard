<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { AlbumTuple, ArtistTuple, StorageStats } from '$lib/types';
	import AlbumCard from '$lib/components/AlbumCard.svelte';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import FilterBar from '$lib/components/FilterBar.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import SkeletonCard from '$lib/components/SkeletonCard.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { formatBytes, formatNumber } from '$lib/format';

	let albums: AlbumTuple[] = [];
	let artists: ArtistTuple[] = [];
	let storage: StorageStats;
	let loading = true;
	let error = '';
	let filter = '';
	let sort = 'title';

	async function load() {
		loading = true;
		error = '';
		try {
			[albums, artists, storage] = await Promise.all([api.albums(), api.artists(), api.storage()]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load albums';
		} finally {
			loading = false;
		}
	}

	$: artistMap = new Map(artists.map(([id, name]) => [id, name]));
	$: filtered = albums
		.filter((album) => `${album[1]} ${artistMap.get(album[2] ?? -1) ?? ''} ${album[4] ?? ''}`.toLowerCase().includes(filter.toLowerCase()))
		.sort((a, b) => sort === 'year' ? (b[3] ?? 0) - (a[3] ?? 0) : a[1].localeCompare(b[1]));
	$: largest = storage?.largest_albums.slice(0, 12) ?? [];
</script>

{#if loading}
	<div class="grid"><SkeletonCard /><SkeletonCard /><SkeletonCard /></div>
{:else if error}
	<ErrorState message={error} retry={load} />
{:else}
	<section class="metric-grid">
		<StatCard label="Albums" value={formatNumber(albums.length)} meta="visible library set" />
		<StatCard label="Largest Album" value={formatBytes(largest[0]?.[3] ?? 0)} meta={largest[0]?.[1] ?? '—'} />
		<StatCard label="Decades" value={formatNumber(new Set(albums.map((a) => Math.floor((a[3] ?? 0) / 10) * 10)).size)} meta="from metadata" />
	</section>

	<div class="toolbar">
		<FilterBar bind:value={filter} placeholder="Filter albums, artists, genres" />
		<select bind:value={sort}>
			<option value="title">Title</option>
			<option value="year">Year</option>
		</select>
	</div>

	<section class="dashboard-grid">
		<ChartCard
			title="Largest Albums"
			option={{
				xAxis: { type: 'value', axisLabel: { color: '#8a8a8a' }, splitLine: { lineStyle: { color: '#262626' } } },
				yAxis: { type: 'category', data: largest.slice(0, 8).map((a) => a[1] ?? 'Unknown').reverse(), axisLabel: { color: '#a3a3a3' } },
				series: [{ type: 'bar', data: largest.slice(0, 8).map((a) => a[3]).reverse(), color: '#e5e5e5' }]
			}}
		/>
		<DataTable columns={['Album', 'Artist', 'Size', 'Tracks']} rows={largest.slice(0, 8).map((a) => [a[1], artistMap.get(a[2] ?? -1) ?? 'Unknown', formatBytes(a[3]), a[4]])} />
	</section>

	<SectionHeader title="Album Grid" eyebrow={`${filtered.length} matches`} />
	{#if filtered.length}
		<div class="media-grid">
			{#each filtered as album}
				<AlbumCard id={album[0]} title={album[1]} artist={artistMap.get(album[2] ?? -1) ?? 'Unknown artist'} year={album[3]} genre={album[4]} />
			{/each}
		</div>
	{:else}
		<EmptyState title="No albums match" />
	{/if}
{/if}
