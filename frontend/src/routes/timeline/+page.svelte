<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	let timeline: [string, number][] = [];
	let error = '';
	let loading = true;
	async function load() {
		loading = true;
		try {
			timeline = await api.timeline();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load timeline';
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
{:else if !timeline.length}
	<EmptyState title="No timestamped plays" detail="Timeline charts require rows in the plays table." />
{:else}
	<ChartCard
		title="Listening Over Time"
		option={{
			xAxis: { type: 'category', data: timeline.map((row) => row[0]), axisLabel: { color: '#8a8a8a' } },
			yAxis: { type: 'value', axisLabel: { color: '#8a8a8a' }, splitLine: { lineStyle: { color: '#262626' } } },
			series: [{ type: 'line', data: timeline.map((row) => row[1]), color: '#f5f5f5', showSymbol: false }]
		}}
	/>
{/if}
