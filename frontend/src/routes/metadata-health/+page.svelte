<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { MetadataHealth, Overview } from '$lib/types';
	import DataTable from '$lib/components/DataTable.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { formatNumber } from '$lib/format';

	let metadata: MetadataHealth;
	let overview: Overview;
	let loading = true;
	let error = '';
	async function load() {
		loading = true;
		try {
			[metadata, overview] = await Promise.all([api.metadataHealth(), api.overview()]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load diagnostics';
		} finally {
			loading = false;
		}
	}
	onMount(load);
	$: score = metadata ? Math.round(((metadata.total_tracks - metadata.missing_genre) / Math.max(metadata.total_tracks, 1)) * 100) : 0;
</script>

{#if loading}
	<div class="skeleton-card"></div>
{:else if error}
	<ErrorState message={error} retry={load} />
{:else}
	<section class="diagnostic-hero">
		<div>
			<p class="eyebrow">Metadata Health Score</p>
			<strong>{score}%</strong>
			<div class="progress"><span style={`width:${score}%`}></span></div>
		</div>
	</section>
	<section class="metric-grid">
		<StatCard label="Missing Genre" value={formatNumber(metadata.missing_genre)} meta={`${formatNumber(metadata.total_tracks)} total tracks`} />
		<StatCard label="Missing MBID" value={formatNumber(metadata.missing_mbid)} meta="track identifiers" />
		<StatCard label="Missing Cover Art" value="—" meta="not exposed by current endpoint" />
		<StatCard label="Timeline Rows" value={formatNumber(overview.total_plays)} meta="plays table" />
	</section>
	<DataTable
		columns={['Task', 'Count', 'Priority']}
		rows={[
			['Add genre tags', metadata.missing_genre, metadata.missing_genre ? 'High' : 'Clear'],
			['Add MusicBrainz IDs', metadata.missing_mbid, metadata.missing_mbid ? 'High' : 'Clear'],
			['Import timestamped plays', overview.total_plays, overview.total_plays ? 'Clear' : 'Optional']
		]}
	/>
{/if}
