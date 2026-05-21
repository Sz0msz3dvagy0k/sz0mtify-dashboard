<script lang="ts">
	import Badge from './Badge.svelte';
	import ImageWithFallback from './ImageWithFallback.svelte';
	import { formatPercent } from '$lib/format';
	import type { DiscoveryItem } from '$lib/types';

	export let item: DiscoveryItem;
</script>

<article class="discovery-card">
	<div class="discovery-image">
		<ImageWithFallback src={item.cover_url} alt={item.title ?? item.discovered_artist_name ?? 'Discovery'} />
	</div>
	<div>
		<div class="discovery-meta">
			<Badge label={item.match_status ?? 'unknown'} dotted={item.match_status === 'possibly_in_library'} />
			<span>{formatPercent(item.confidence_score, 100)} confidence</span>
		</div>
		<h3 class={item.title ? '' : 'artist-name'}>{item.title ?? item.discovered_artist_name}</h3>
		<p class="artist-name">{item.discovered_artist_name ?? item.local_artist_name}</p>
		{#if item.reason}<small>{item.reason}</small>{/if}
		{#if item.external_url}<a class="text-link" href={item.external_url} target="_blank" rel="noreferrer">Last.fm</a>{/if}
	</div>
</article>
