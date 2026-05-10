<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { StorageStats } from '$lib/types';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
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
			option={{ series: [{ type: 'treemap', roam: false, breadcrumb: { show: false }, data: storage.size_by_artist.slice(0, 30).map(([_, name, bytes]) => ({ name: name ?? 'Unknown', value: bytes })), color: ['#f5f5f5', '#cfcfcf', '#9f9f9f', '#6f6f6f', '#404040'] }] }}
			height={360}
		/>
		<ChartCard
			title="Extension Breakdown"
			option={{ series: [{ type: 'pie', radius: ['45%', '72%'], data: storage.extension_breakdown.map(([name, tracks]) => ({ name: name ?? 'unknown', value: tracks })), color: ['#f5f5f5', '#8a8a8a', '#525252'] }] }}
		/>
	</section>
	<DataTable columns={['Largest Album', 'Size', 'Tracks']} rows={storage.largest_albums.slice(0, 12).map((a) => [a[1], formatBytes(a[3]), a[4]])} />
	<DataTable columns={['Largest Track', 'Size', 'Format']} rows={storage.largest_tracks.slice(0, 12).map((t) => [t[1], formatBytes(t[4]), t[6] ?? t[7] ?? '—'])} />
	{#if storage.suspicious_large_tracks.length}
		<DataTable columns={['Suspicious Track', 'Size', 'Format']} rows={storage.suspicious_large_tracks.map((t) => [t[1], formatBytes(t[2]), t[4] ?? '—'])} />
	{:else}
		<EmptyState title="No suspicious large files" />
	{/if}
{/if}
