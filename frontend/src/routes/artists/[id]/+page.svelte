<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { ArtistDetail, StorageStats } from '$lib/types';
	import AlbumCard from '$lib/components/AlbumCard.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ImageWithFallback from '$lib/components/ImageWithFallback.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { formatBytes, formatNumber } from '$lib/format';

	let detail: ArtistDetail;
	let storage: StorageStats;
	let error = '';
	let loading = true;

	async function load() {
		loading = true;
		try {
			[detail, storage] = await Promise.all([api.artist(Number($page.params.id)), api.storage()]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load artist';
		} finally {
			loading = false;
		}
	}
	onMount(load);
	$: artist = detail?.artist;
	$: artistStorage = storage?.size_by_artist.find((row) => row[0] === artist?.[0]);
</script>

{#if loading}
	<div class="skeleton-card"></div>
{:else if error}
	<ErrorState message={error} retry={load} />
{:else if !artist}
	<EmptyState title="Artist not found" />
{:else}
	<section class="detail-hero">
		<div class="detail-art">
			<ImageWithFallback alt={artist[1]} kind="artist" />
		</div>
		<div>
			<p class="eyebrow">Artist profile</p>
			<h2>{artist[1]}</h2>
			<div class="metric-grid compact">
				<StatCard label="Albums" value={formatNumber(artist[2] ?? 0)} />
				<StatCard label="Tracks" value={formatNumber(artist[3] ?? 0)} />
				<StatCard label="Storage" value={formatBytes(artistStorage?.[2] ?? 0)} />
			</div>
			{#if artist[5]}<p class="bio">{@html artist[5]}</p>{/if}
		</div>
	</section>
	<div class="media-grid">
		{#each detail.albums as album}
			<AlbumCard id={album[0]} title={album[1]} artist={artist[1]} year={album[2]} />
		{/each}
	</div>
{/if}
