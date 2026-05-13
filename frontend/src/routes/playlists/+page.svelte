<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { PlaylistSummary } from '$lib/types';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ImageWithFallback from '$lib/components/ImageWithFallback.svelte';
	import ItemsPerPage from '$lib/components/ItemsPerPage.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import SkeletonCard from '$lib/components/SkeletonCard.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { coverUrl, formatDuration, formatNumber } from '$lib/format';

	let playlists: PlaylistSummary[] = [];
	let loading = true;
	let error = '';
	let itemsPerPage = 18;
	let page = 1;

	async function load() {
		loading = true;
		error = '';
		try {
			playlists = await api.playlists();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load playlists';
		} finally {
			loading = false;
		}
	}

	onMount(load);
	$: pageStart = (page - 1) * itemsPerPage;
	$: visiblePlaylists = playlists.slice(pageStart, pageStart + itemsPerPage);
	$: if (page > Math.max(1, Math.ceil(playlists.length / itemsPerPage))) page = 1;
</script>

{#if loading}
	<div class="grid"><SkeletonCard /><SkeletonCard /><SkeletonCard /></div>
{:else if error}
	<ErrorState message={error} retry={load} />
{:else}
	<section class="metric-grid">
		<StatCard label="Playlists" value={formatNumber(playlists.length)} />
		<StatCard label="Tracks" value={formatNumber(playlists.reduce((sum, playlist) => sum + playlist.song_count, 0))} />
		<StatCard label="Longest" value={formatDuration(Math.max(...playlists.map((playlist) => playlist.duration_seconds), 0))} />
	</section>

	<SectionHeader title="Playlists" eyebrow="Navidrome" />
	{#if visiblePlaylists.length}
		<div class="media-grid">
			{#each visiblePlaylists as playlist}
				<a class="media-card" href={`/playlists/${encodeURIComponent(playlist.id)}`}>
					<div class="media-cover">
						<ImageWithFallback src={coverUrl(playlist.cover_art_id)} alt={playlist.name} />
					</div>
					<div class="media-body">
						<strong>{playlist.name}</strong>
						<span>{formatNumber(playlist.song_count)} tracks</span>
						<em>{formatDuration(playlist.duration_seconds)}</em>
					</div>
				</a>
			{/each}
		</div>
		<ItemsPerPage bind:value={itemsPerPage} bind:page total={playlists.length} shown={visiblePlaylists.length} />
	{:else}
		<EmptyState title="No playlists found" />
	{/if}
{/if}
