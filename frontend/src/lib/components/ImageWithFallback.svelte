<script lang="ts">
	import { Disc3, UserRound } from 'lucide-svelte';
	import { initials } from '$lib/format';

	export let src: string | null = null;
	export let alt = '';
	export let kind: 'album' | 'artist' = 'album';
	let failed = false;
</script>

{#if src && !failed}
	<img class="art-image" {src} {alt} loading="lazy" on:error={() => (failed = true)} />
{:else}
	<div class:artist={kind === 'artist'} class="art-fallback" aria-label={alt}>
		{#if kind === 'artist'}
			<UserRound size={28} strokeWidth={1.5} />
		{:else}
			<Disc3 size={28} strokeWidth={1.5} />
		{/if}
		<span>{initials(alt)}</span>
	</div>
{/if}
