<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { GenreTuple, StorageStats } from '$lib/types';
	import Badge from '$lib/components/Badge.svelte';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { formatBytes, formatNumber } from '$lib/format';

	let genres: GenreTuple[] = [];
	let storage: StorageStats;
	let error = '';
	let loading = true;
	async function load() {
		loading = true;
		try {
			[genres, storage] = await Promise.all([api.genres(), api.storage()]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load genres';
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
	<section class="metric-grid">
		<StatCard label="Genres" value={formatNumber(genres.length)} />
		<StatCard label="Largest Genre" value={storage.size_by_genre[0]?.[0] ?? 'Unknown'} meta={formatBytes(storage.size_by_genre[0]?.[1] ?? 0)} />
		<StatCard label="Mood Tags" value="0" meta="no mood data" />
	</section>
	<section class="dashboard-grid">
		<ChartCard
			title="Genre Distribution"
			option={{
				series: [{ type: 'treemap', roam: false, breadcrumb: { show: false }, data: storage.size_by_genre.slice(0, 24).map(([name, bytes]) => ({ name: name ?? 'Unknown', value: bytes })), color: ['#f5f5f5', '#cfcfcf', '#9f9f9f', '#6f6f6f', '#404040'] }]
			}}
			height={340}
		/>
		<DataTable columns={['Genre', 'Tracks', 'Albums', 'Artists']} rows={genres.slice(0, 18).map((g) => [g[1], g[2] ?? 0, g[3] ?? 0, g[4] ?? 0])} />
	</section>
	<SectionHeader title="Genre Tags" eyebrow="library taxonomy" />
	<div class="badge-cloud">
		{#each genres as genre}
			<Badge label={`${genre[1]} · ${genre[2] ?? 0}`} />
		{/each}
	</div>
	<EmptyState title="No mood tags found" detail="Mood analysis will appear when tracks include mood metadata." />
{/if}
