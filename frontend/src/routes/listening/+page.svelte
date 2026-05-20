<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { ListeningStats } from '$lib/types';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import TrackRow from '$lib/components/TrackRow.svelte';
	import { formatNumber } from '$lib/format';
	import { trackHref } from '$lib/navigation';

	let listening: ListeningStats;
	let rotation: [number, string, string | null, number | null, number | null][] = [];
	let rediscovery: { tracks: [number, string, string | null, number | null, number, string | null][]; score_example: number };
	let loading = true;
	let error = '';
	async function load() {
		loading = true;
		try {
			[listening, rotation, rediscovery] = await Promise.all([api.listening(), api.currentRotation(), api.rediscovery()]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load listening stats';
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
		<StatCard label="Total Plays" value={formatNumber(listening.total_plays)} meta={listening.data_source.replaceAll('_', ' ')} />
		<StatCard label="Top Tracks" value={formatNumber(listening.top_tracks.length)} />
		<StatCard label="Timestamp Events" value={listening.has_play_events ? 'Yes' : 'No'} />
	</section>
	{#if !listening.has_play_events}
		<EmptyState title="Using imported Subsonic play counts" detail="Timeline requires timestamped play events." />
	{/if}
	<section class="dashboard-grid">
		<ChartCard
			title="Top Artists"
			option={{
				xAxis: { type: 'value', axisLabel: { color: '#8a8a8a' }, splitLine: { lineStyle: { color: '#262626' } } },
				yAxis: { type: 'category', data: listening.top_artists.slice(0, 10).map((a) => a[1] ?? 'Unknown').reverse(), axisLabel: { color: '#a3a3a3' } },
				series: [{ type: 'bar', data: listening.top_artists.slice(0, 10).map((a) => a[2]).reverse(), color: '#f5f5f5' }]
			}}
		/>
		<ChartCard
			title="Top Albums"
			option={{
				xAxis: { type: 'value', axisLabel: { color: '#8a8a8a' }, splitLine: { lineStyle: { color: '#262626' } } },
				yAxis: { type: 'category', data: listening.top_albums.slice(0, 10).map((a) => a[1] ?? 'Unknown').reverse(), axisLabel: { color: '#a3a3a3' } },
				series: [{ type: 'bar', data: listening.top_albums.slice(0, 10).map((a) => a[2]).reverse(), color: '#d4d4d4' }]
			}}
		/>
	</section>
	<section class="split-grid">
		<div>
			<SectionHeader title="Top Tracks" eyebrow="play count" />
			<div class="panel-list">{#each listening.top_tracks.slice(0, 12) as track}<TrackRow title={track[1]} count={track[3]} href={trackHref(track[0])} />{/each}</div>
		</div>
		<div>
			<SectionHeader title="Current Rotation" eyebrow="recent weighting" />
			<div class="panel-list">{#each rotation.slice(0, 12) as track}<TrackRow title={track[1]} detail={track[2] ?? ''} count={track[4] ?? 0} href={trackHref(track[0])} />{/each}</div>
		</div>
	</section>
	<section>
		<SectionHeader title="Rediscovery Candidates" eyebrow="deep cuts" />
		<div class="panel-list">{#each rediscovery.tracks.slice(0, 10) as track}<TrackRow title={track[1]} detail={track[5] ?? ''} count={track[4]} href={trackHref(track[0])} />{/each}</div>
	</section>
{/if}
