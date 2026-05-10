<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { StorageStats } from '$lib/types';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { formatBytes, formatNumber } from '$lib/format';

	let quality: [number | null, number | null, number][] = [];
	let storage: StorageStats;
	let loading = true;
	let error = '';
	async function load() {
		loading = true;
		try {
			[quality, storage] = await Promise.all([api.audioQuality(), api.storage()]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load quality data';
		} finally {
			loading = false;
		}
	}
	onMount(load);
	$: lossless = storage?.size_by_format.find((row) => row[0] === 'flac')?.[2] ?? 0;
	$: hires = quality.filter(([rate, depth]) => (rate ?? 0) > 48000 || (depth ?? 0) > 16).reduce((sum, row) => sum + row[2], 0);
	$: heatData = quality.map(([rate, depth, count]) => [String(rate ?? 'unknown'), String(depth ?? 'unknown'), count]);
</script>

{#if loading}
	<div class="skeleton-card"></div>
{:else if error}
	<ErrorState message={error} retry={load} />
{:else}
	<section class="metric-grid">
		<StatCard label="FLAC Tracks" value={formatNumber(lossless)} meta="lossless files" />
		<StatCard label="Hi-res Tracks" value={formatNumber(hires)} meta="above CD quality" />
		<StatCard label="Quality Groups" value={formatNumber(quality.length)} meta="bitrate × depth" />
	</section>
	<section class="dashboard-grid">
		<ChartCard
			title="Format Distribution"
			option={{ series: [{ type: 'pie', radius: ['45%', '72%'], data: storage.size_by_format.map(([name, bytes]) => ({ name: name ?? 'unknown', value: bytes })), color: ['#f5f5f5', '#8a8a8a', '#525252'] }] }}
		/>
		<ChartCard
			title="Bitrate Ranking"
			option={{
				xAxis: { type: 'category', data: quality.slice(0, 16).map((q) => `${q[0] ?? '—'} kbps`), axisLabel: { color: '#8a8a8a', rotate: 45 } },
				yAxis: { type: 'value', axisLabel: { color: '#8a8a8a' }, splitLine: { lineStyle: { color: '#262626' } } },
				series: [{ type: 'bar', data: quality.slice(0, 16).map((q) => q[2]), color: '#f5f5f5' }]
			}}
		/>
	</section>
	<DataTable columns={['Bitrate', 'Bit depth', 'Tracks']} rows={quality.slice(0, 40).map((q) => [q[0] ? `${q[0]} kbps` : 'Unknown', q[1] ? `${q[1]} bit` : 'Unknown', q[2]])} />
	<DataTable columns={['Track', 'Size', 'Duration', 'Format']} rows={storage.largest_tracks.slice(0, 12).map((t) => [t[1], formatBytes(t[4]), t[5] ?? '—', t[6] ?? t[7] ?? '—'])} />
{/if}
