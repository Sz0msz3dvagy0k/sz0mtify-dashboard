<script lang="ts">
	import { page } from '$app/stores';
	import { onDestroy, onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { AlbumDetail, ArtistTuple } from '$lib/types';
	import DataTable from '$lib/components/DataTable.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ImageWithFallback from '$lib/components/ImageWithFallback.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { coverUrl, formatNumber } from '$lib/format';

	let detail: AlbumDetail;
	let artists: ArtistTuple[] = [];
	let error = '';
	let loading = true;
	let highlightedTrackId: number | null = null;
	let lastHighlightedTrackParam: number | null = null;
	let highlightTimer: ReturnType<typeof setTimeout> | null = null;

	async function load() {
		loading = true;
		try {
			[detail, artists] = await Promise.all([api.album(Number($page.params.id)), api.artists()]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load album';
		} finally {
			loading = false;
		}
	}
	onMount(load);
	onDestroy(() => {
		if (highlightTimer) clearTimeout(highlightTimer);
	});
	$: album = detail?.album;
	$: artistName = artists.find((artist) => artist[0] === album?.[2])?.[1] ?? 'Unknown artist';
	$: trackParam = Number($page.url.searchParams.get('track'));
	$: if (!loading && Number.isFinite(trackParam) && trackParam > 0 && trackParam !== lastHighlightedTrackParam) {
		lastHighlightedTrackParam = trackParam;
		highlightedTrackId = trackParam;
		if (highlightTimer) clearTimeout(highlightTimer);
		highlightTimer = setTimeout(() => {
			highlightedTrackId = null;
			highlightTimer = null;
		}, 1200);
	}
</script>

{#if loading}
	<div class="skeleton-card"></div>
{:else if error}
	<ErrorState message={error} retry={load} />
{:else if !album}
	<EmptyState title="Album not found" />
{:else}
	<section class="detail-hero">
		<div class="detail-art">
			<ImageWithFallback src={coverUrl(album[6])} alt={album[1]} />
		</div>
		<div>
			<p class="eyebrow">{artistName}</p>
			<h2>{album[1]}</h2>
			<div class="metric-grid compact">
				<StatCard label="Year" value={album[3] ?? '—'} />
				<StatCard label="Genre" value={album[4] ?? '—'} />
				<StatCard label="Tracks" value={formatNumber(detail.tracks.length)} />
			</div>
		</div>
	</section>
	<DataTable
		columns={['#', 'Track', 'Disc']}
		rows={detail.tracks.map((track) => [track[2] ?? '—', track[1], track[3] ?? 1])}
		rowKeys={detail.tracks.map((track) => track[0])}
		highlightedRowKey={highlightedTrackId}
	/>
{/if}
