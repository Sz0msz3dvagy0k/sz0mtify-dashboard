<script lang="ts">
	import { api } from '$lib/api';
	import type { SearchResult } from '$lib/types';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import { albumTrackHref } from '$lib/navigation';

	let q = '';
	let result: SearchResult | null = null;
	let error = '';
	let loading = false;

	async function search() {
		error = '';
		result = null;
		if (!q.trim()) return;
		loading = true;
		try {
			result = await api.search(q);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Search failed';
		} finally {
			loading = false;
		}
	}
</script>

<section class="search-panel">
	<input bind:value={q} on:keydown={(event) => event.key === 'Enter' && search()} placeholder="Search tracks, albums, artists" autofocus />
	<button class="button" on:click={search} disabled={loading}>{loading ? 'Searching…' : 'Search'}</button>
</section>

{#if error}
	<ErrorState message={error} retry={search} />
{:else if result}
	<section class="split-grid">
		<div>
			<SectionHeader title="Tracks" eyebrow={`${result.tracks.length} results`} />
			<div class="panel-list">
				{#each result.tracks as row}
					{#if albumTrackHref(row[0], row[2])}
						<a class="result-row" href={albumTrackHref(row[0], row[2])}>{row[1]}</a>
					{:else}
						<div class="result-row">{row[1]}</div>
					{/if}
				{/each}
			</div>
		</div>
		<div>
			<SectionHeader title="Albums" eyebrow={`${result.albums.length} results`} />
			<div class="panel-list">{#each result.albums as row}<a class="result-row" href={`/albums/${row[0]}`}>{row[1]}</a>{/each}</div>
			<SectionHeader title="Artists" eyebrow={`${result.artists.length} results`} />
			<div class="panel-list">{#each result.artists as row}<a class="result-row" href={`/artists/${row[0]}`}>{row[1]}</a>{/each}</div>
		</div>
	</section>
{:else}
	<EmptyState title="Search the archive" />
{/if}
