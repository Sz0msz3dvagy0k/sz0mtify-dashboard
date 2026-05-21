<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { ArtistDetail, StorageStats } from '$lib/types';
	import AlbumCard from '$lib/components/AlbumCard.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ImageWithFallback from '$lib/components/ImageWithFallback.svelte';
	import ItemsPerPage from '$lib/components/ItemsPerPage.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { coverUrl, formatArtistBio, formatBytes, formatNumber } from '$lib/format';

	let detail: ArtistDetail;
	let storage: StorageStats;
	let resolvedArtistCoverArtId: string | null = null;
	let error = '';
	let loading = true;
	let itemsPerPage = 18;
	let pageIndex = 1;

	async function load() {
		loading = true;
		resolvedArtistCoverArtId = null;
		try {
			[detail, storage] = await Promise.all([api.artist(Number($page.params.id)), api.storage()]);
			if (!detail.albums.some((album) => album[3])) {
				const albumId = detail.albums[0]?.[0];
				if (albumId) {
					const albumDetail = await api.album(albumId);
					resolvedArtistCoverArtId = albumDetail.album?.[6] ?? null;
				}
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load artist';
		} finally {
			loading = false;
		}
	}
	onMount(load);
	$: artist = detail?.artist;
	$: artistStorage = storage?.size_by_artist.find((row) => row[0] === artist?.[0]);
	$: representativeCoverArtId = detail?.albums.find((album) => album[3])?.[3] ?? resolvedArtistCoverArtId;
	$: artistImageUrl = artist?.[6] ?? null;
	$: fallbackHeroImageSrc = coverUrl(representativeCoverArtId);
	$: artistBio = formatArtistBio(artist?.[5]);
	$: pageStart = (pageIndex - 1) * itemsPerPage;
	$: visibleAlbums = detail?.albums.slice(pageStart, pageStart + itemsPerPage) ?? [];
	$: if (pageIndex > Math.max(1, Math.ceil((detail?.albums.length ?? 0) / itemsPerPage))) pageIndex = 1;
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
			<ImageWithFallback src={artistImageUrl} fallbackSrc={fallbackHeroImageSrc} alt={artist[1]} kind="artist" />
		</div>
		<div>
			<p class="eyebrow">Artist profile</p>
			<h2 class="artist-name">{artist[1]}</h2>
			<div class="metric-grid compact">
				<StatCard label="Albums" value={formatNumber(artist[2] ?? 0)} />
				<StatCard label="Tracks" value={formatNumber(artist[3] ?? 0)} />
				<StatCard label="Storage" value={formatBytes(artistStorage?.[2] ?? 0)} />
			</div>
			{#if artistBio}<p class="bio">{artistBio}</p>{/if}
		</div>
	</section>
	<div class="media-grid">
		{#each visibleAlbums as album}
			<AlbumCard id={album[0]} title={album[1]} artist={artist[1]} year={album[2]} coverArtId={album[3]} />
		{/each}
	</div>
	<ItemsPerPage bind:value={itemsPerPage} bind:page={pageIndex} total={detail.albums.length} shown={visibleAlbums.length} />
{/if}
