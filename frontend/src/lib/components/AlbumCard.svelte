<script lang="ts">
	import ImageWithFallback from './ImageWithFallback.svelte';
	import { coverUrl } from '$lib/format';

	export let id: number;
	export let title: string;
	export let artist = 'Unknown artist';
	export let year: number | null = null;
	export let genre: string | null = null;
	export let coverArtId: string | null = null;
</script>

<a class="media-card" href={`/albums/${id}`}>
	<div class="media-cover">
		<ImageWithFallback src={coverUrl(coverArtId)} alt={title} />
	</div>
	<div class="media-body">
		<strong>{title}</strong>
		<span>{artist}{year ? ` • ${year}` : ''}</span>
		{#if genre}<em>{genre}</em>{/if}
	</div>
</a>
