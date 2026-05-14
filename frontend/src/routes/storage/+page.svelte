<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { StorageStats } from '$lib/types';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ExpandableTable from '$lib/components/ExpandableTable.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { formatBytes } from '$lib/format';

	let storage: StorageStats;
	let loading = true;
	let error = '';
	async function load() {
		loading = true;
		try {
			storage = await api.storage();
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
