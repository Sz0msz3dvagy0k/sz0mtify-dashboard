<script lang="ts">
	import { browser } from '$app/environment';
	import { goto } from '$app/navigation';
	import { page as routePage } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { AlbumTuple, ArtistTuple, StorageStats } from '$lib/types';
	import AlbumCard from '$lib/components/AlbumCard.svelte';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ExpandableTable from '$lib/components/ExpandableTable.svelte';
	import FilterBar from '$lib/components/FilterBar.svelte';
	import ItemsPerPage from '$lib/components/ItemsPerPage.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import SkeletonCard from '$lib/components/SkeletonCard.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { formatBytes, formatNumber } from '$lib/format';

	const albumSortStorageKey = 'archive.albums.sort';
	const albumSortOptions = new Set([
		'title',
		'title-desc',
		'artist',
		'year-new',
		'year-old',
		'date-added-new',
		'date-added-old',
		'genre',
		'size',
		'tracks'
	]);

	let albums: AlbumTuple[] = [];
	let artists: ArtistTuple[] = [];
	let storage: StorageStats | null = null;
	let loading = true;
	let error = '';
	let filter = '';
	let sort = storedAlbumSort();
	let itemsPerPage = 18;
	let page = 1;
	let syncedPage = 1;
	let pendingPage: number | null = null;

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
	$: albumStorageMap = new Map((storage?.size_by_album ?? []).filter((row) => row[0] !== null).map((row) => [row[0] as number, { bytes: row[3], tracks: row[4] }]));
	$: filtered = albums
		.filter((album) => `${album[1]} ${albumArtistName(album)} ${album[4] ?? ''}`.toLowerCase().includes(filter.toLowerCase()))
		.sort((a, b) => {
			switch (sort) {
				case 'title-desc':
					return b[1].localeCompare(a[1]);
				case 'artist':
					return albumArtistName(a).localeCompare(albumArtistName(b)) || a[1].localeCompare(b[1]);
				case 'year-new':
					return (b[3] ?? 0) - (a[3] ?? 0) || a[1].localeCompare(b[1]);
				case 'year-old':
					return (a[3] ?? 9999) - (b[3] ?? 9999) || a[1].localeCompare(b[1]);
				case 'date-added-new':
					return compareAddedNewest(a, b);
				case 'date-added-old':
					return compareAddedOldest(a, b);
				case 'genre':
					return (a[4] ?? '').localeCompare(b[4] ?? '') || a[1].localeCompare(b[1]);
				case 'size':
					return (albumStorageMap.get(b[0])?.bytes ?? 0) - (albumStorageMap.get(a[0])?.bytes ?? 0);
				case 'tracks':
					return (albumStorageMap.get(b[0])?.tracks ?? 0) - (albumStorageMap.get(a[0])?.tracks ?? 0);
				default:
					return a[1].localeCompare(b[1]);
			}
		});
	$: largest = storage?.largest_albums.slice(0, 12) ?? [];
	$: largestChartRows = largest.slice(0, 8);
	$: pageStart = (page - 1) * itemsPerPage;
	$: visibleAlbums = filtered.slice(pageStart, pageStart + itemsPerPage);
	$: largestTableRows = largest.slice(0, 8).map((a, index) => ({
		id: a[0] ?? `album-${index}`,
		title: a[1] ?? 'Unknown album',
		href: a[0] ? detailHref(`/albums/${a[0]}`) : null,
		details: [
			['Artist', artistMap.get(a[2] ?? -1) ?? 'Unknown'],
			['Size', formatBytes(a[3])],
			['Tracks', a[4]]
		] as [string, string | number | null | undefined][]
	}));
	$: if (page > Math.max(1, Math.ceil(filtered.length / itemsPerPage))) page = 1;
	$: urlPage = pageFromUrl();
	$: if (pendingPage !== null && urlPage === pendingPage) pendingPage = null;
	$: if (pendingPage === null && urlPage !== syncedPage) {
		syncedPage = urlPage;
		page = urlPage;
	}

	onMount(load);

	function pageFromUrl() {
		const value = Number($routePage.url.searchParams.get('page'));
		return Number.isFinite(value) && value > 0 ? Math.floor(value) : 1;
	}

	function pageUrl(index: number) {
		const url = new URL($routePage.url);
		if (index <= 1) {
			url.searchParams.delete('page');
		} else {
			url.searchParams.set('page', String(index));
		}
		return `${url.pathname}${url.search}${url.hash}`;
	}

	function detailHref(path: string) {
		return `${path}?from=${encodeURIComponent(pageUrl(page))}`;
	}

	function rememberPage(nextPage: number) {
		if (!browser) return;
		syncedPage = nextPage;
		pendingPage = nextPage;
		void goto(pageUrl(nextPage), { replaceState: true, noScroll: true, keepFocus: true });
	}

	function albumArtistName(album: AlbumTuple): string {
		return album[6] ?? artistMap.get(album[2] ?? -1) ?? 'Unknown artist';
	}

	function albumAddedTime(album: AlbumTuple): number {
		const value = album[7];
		if (!value) return 0;
		const time = Date.parse(value);
		return Number.isNaN(time) ? 0 : time;
	}

	function compareAddedNewest(a: AlbumTuple, b: AlbumTuple): number {
		return albumAddedTime(b) - albumAddedTime(a) || b[0] - a[0] || a[1].localeCompare(b[1]);
	}

	function compareAddedOldest(a: AlbumTuple, b: AlbumTuple): number {
		const aTime = albumAddedTime(a);
		const bTime = albumAddedTime(b);
		if (!aTime && !bTime) return a[1].localeCompare(b[1]);
		if (!aTime) return 1;
		if (!bTime) return -1;
		return aTime - bTime || a[0] - b[0] || a[1].localeCompare(b[1]);
	}

	function storedAlbumSort() {
		if (!browser) return 'title';
		const value = localStorage.getItem(albumSortStorageKey);
		return value && albumSortOptions.has(value) ? value : 'title';
	}

	function rememberAlbumSort() {
		if (!browser) return;
		localStorage.setItem(albumSortStorageKey, sort);
	}
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
		<select bind:value={sort} on:change={rememberAlbumSort}>
			<option value="title">Title A-Z</option>
			<option value="title-desc">Title Z-A</option>
			<option value="artist">Artist</option>
			<option value="year-new">Newest Year</option>
			<option value="year-old">Oldest Year</option>
			<option value="date-added-new">Recently Added</option>
			<option value="date-added-old">Oldest Added</option>
			<option value="genre">Genre</option>
			<option value="size">Largest Size</option>
			<option value="tracks">Most Tracks</option>
		</select>
	</div>

	<section class="dashboard-grid">
		<ChartCard
			title="Largest Albums"
			option={{
				xAxis: { type: 'value', axisLabel: { color: '#8a8a8a', formatter: (value: number) => `${value} MB` }, splitLine: { lineStyle: { color: '#262626' } } },
				yAxis: { type: 'category', data: largestChartRows.map((a) => a[1] ?? 'Unknown').reverse(), axisLabel: { color: '#a3a3a3' } },
				series: [{ type: 'bar', data: largestChartRows.map((a) => Number((a[3] / 1024 ** 2).toFixed(1))).reverse(), color: '#e5e5e5' }],
				tooltip: { valueFormatter: (value: number) => `${value.toFixed(1)} MB` }
			}}
		/>
		<ExpandableTable title="Largest Albums" rows={largestTableRows} />
	</section>

	<SectionHeader title="Album Grid" eyebrow={`${filtered.length} matches`} />
	{#if filtered.length}
		<div class="media-grid">
			{#each visibleAlbums as album}
				<AlbumCard id={album[0]} title={album[1]} artist={albumArtistName(album)} year={album[3]} genre={album[4]} coverArtId={album[5]} href={detailHref(`/albums/${album[0]}`)} />
			{/each}
		</div>
		<ItemsPerPage bind:value={itemsPerPage} bind:page total={filtered.length} shown={visibleAlbums.length} onPageChange={rememberPage} />
	{:else}
		<EmptyState title="No albums match" />
	{/if}
{/if}
