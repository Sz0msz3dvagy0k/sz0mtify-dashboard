<script lang="ts">
	import { browser } from '$app/environment';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import type { SearchResult } from '$lib/types';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ImageWithFallback from '$lib/components/ImageWithFallback.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import { coverUrl, formatDuration } from '$lib/format';
	import { player, playQueue, type QueueTrack } from '$lib/player';

	let query = '';
	let loadedQuery = '';
	let result: SearchResult | null = null;
	let error = '';
	let loading = false;

	$: queryParam = $page.url.searchParams.get('q')?.trim() ?? '';
	$: if (browser && queryParam !== loadedQuery) void search(queryParam);
	$: trackQueue = result ? result.tracks.map(trackToQueueItem) : [];
	$: playingTrackId = $player.isPlaying ? $player.queue[$player.currentIndex]?.id ?? null : null;

	async function search(nextQuery: string) {
		query = nextQuery;
		loadedQuery = nextQuery;
		error = '';
		result = null;
		if (!nextQuery) return;
		loading = true;
		try {
			result = await api.search(nextQuery);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Search failed';
		} finally {
			loading = false;
		}
	}

	function playTrack(index: number) {
		playQueue(trackQueue, index);
	}

	function trackToQueueItem(row: SearchResult['tracks'][number]): QueueTrack {
		return {
			id: row[0],
			title: row[1],
			artist: row[2] ?? 'Unknown artist',
			album: row[4] ?? 'Unknown album',
			albumId: row[3],
			coverArtId: row[5],
			duration: row[6]
		};
	}
</script>

{#if loading}
	<div class="skeleton-card"></div>
{:else if error}
	<ErrorState message={error} retry={() => search(query)} />
{:else if result}
	<section class="split-grid">
		<div>
			<SectionHeader title="Tracks" eyebrow={`${result.tracks.length} results`} />
			<div class="panel-list search-results">
				{#each result.tracks as row, index}
					<button class="search-result-row" class:playing-row={row[0] === playingTrackId} on:click={() => playTrack(index)}>
						<div class="search-result-art">
							<ImageWithFallback src={coverUrl(row[5])} alt={row[1]} />
						</div>
						<span>
							<strong>{row[1]}</strong>
							<small>{row[2] ?? 'Unknown artist'} · {row[4] ?? 'Unknown album'}</small>
						</span>
						<em>{formatDuration(row[6])}</em>
					</button>
				{/each}
			</div>
		</div>
		<div>
			<SectionHeader title="Albums" eyebrow={`${result.albums.length} results`} />
			<div class="panel-list search-results">
				{#each result.albums as row}
					<a class="search-result-row" href={`/albums/${row[0]}`}>
						<div class="search-result-art">
							<ImageWithFallback src={coverUrl(row[3])} alt={row[1]} />
						</div>
						<span>
							<strong>{row[1]}</strong>
							<small>{row[2] ?? 'Unknown artist'}</small>
						</span>
					</a>
				{/each}
			</div>
			<SectionHeader title="Artists" eyebrow={`${result.artists.length} results`} />
			<div class="panel-list search-results">
				{#each result.artists as row}
					<a class="search-result-row" href={`/artists/${row[0]}`}>
						<div class="search-result-art artist">
							<ImageWithFallback src={row[2]} fallbackSrc={coverUrl(row[3])} alt={row[1]} kind="artist" />
						</div>
						<span>
							<strong>{row[1]}</strong>
						</span>
					</a>
				{/each}
			</div>
		</div>
	</section>
{:else}
	<EmptyState title="Search the archive" />
{/if}
