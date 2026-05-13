<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { GenreTuple, StorageStats, TrackTuple } from '$lib/types';
	import Badge from '$lib/components/Badge.svelte';
	import ChartCard from '$lib/components/ChartCard.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import TrackRow from '$lib/components/TrackRow.svelte';
	import { formatBytes, formatNumber } from '$lib/format';
	import { albumTrackHref } from '$lib/navigation';

	let genres: GenreTuple[] = [];
	let tracks: TrackTuple[] = [];
	let storage: StorageStats;
	let error = '';
	let loading = true;
	async function load() {
		loading = true;
		try {
			[genres, storage, tracks] = await Promise.all([api.genres(), api.storage(), api.tracks()]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load genres';
		} finally {
			loading = false;
		}
	}
	onMount(load);
	$: selectedGenre = $page.url.searchParams.get('genre');
	$: selectedGenreTracks = selectedGenre ? tracks.filter((track) => (track[5] ?? 'Unknown') === selectedGenre) : [];
	$: genreRows = genres.slice(0, 18).map((g) => [g[1], g[2] ?? 0, g[3] ?? 0, g[4] ?? 0]);
	$: genreLinks = genres.slice(0, 18).map((g) => [`/genres?genre=${encodeURIComponent(g[1])}`, null, null, null]);
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
		<DataTable columns={['Genre', 'Tracks', 'Albums', 'Artists']} rows={genreRows} cellLinks={genreLinks} />
	</section>
	<SectionHeader title="Genre Tags" eyebrow="library taxonomy" />
	<div class="badge-cloud">
		{#each genres as genre}
			<a class="badge-link" href={`/genres?genre=${encodeURIComponent(genre[1])}`}>
				<Badge label={`${genre[1]} · ${genre[2] ?? 0}`} />
			</a>
		{/each}
	</div>
	{#if selectedGenre}
		<SectionHeader title={selectedGenre} eyebrow={`${selectedGenreTracks.length} songs`} />
		{#if selectedGenreTracks.length}
			<div class="panel-list">
				{#each selectedGenreTracks as track}
					<TrackRow title={track[1]} detail={track[5] ?? ''} duration={track[4]} href={albumTrackHref(track[0], track[3])} />
				{/each}
			</div>
		{:else}
			<EmptyState title="No songs found" />
		{/if}
	{/if}
	<EmptyState title="No mood tags found" detail="Mood analysis will appear when tracks include mood metadata." />
{/if}
