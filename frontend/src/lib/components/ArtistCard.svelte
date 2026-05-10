<script lang="ts">
	import ImageWithFallback from './ImageWithFallback.svelte';
	import { formatNumber } from '$lib/format';

	export let id: number;
	export let name: string;
	export let albums = 0;
	export let tracks = 0;
	export let plays = 0;
</script>

<a class="media-card artist-card" href={`/artists/${id}`}>
	<div class="media-cover">
		<ImageWithFallback alt={name} kind="artist" />
	</div>
	<div class="media-body">
		<strong>{name}</strong>
		<span>{formatNumber(albums)} albums • {formatNumber(tracks)} tracks</span>
		<em>{formatNumber(plays)} plays</em>
	</div>
</a>
