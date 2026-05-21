<script lang="ts">
	import { formatDuration, formatNumber } from '$lib/format';
	export let title = '';
	export let detail = '';
	export let duration: number | null = null;
	export let count: number | null = null;
	export let href: string | null = null;
	export let detailClass = '';
</script>

{#if href}
	<a class="track-row" {href}>
		<div>
			<strong>{title}</strong>
			{#if detail}<span class={detailClass}>{detail}</span>{/if}
		</div>
		<em>{count !== null ? formatNumber(count) : formatDuration(duration)}</em>
	</a>
{:else}
	<div class="track-row">
		<div>
			<strong>{title}</strong>
			{#if detail}<span class={detailClass}>{detail}</span>{/if}
		</div>
		<em>{count !== null ? formatNumber(count) : formatDuration(duration)}</em>
	</div>
{/if}
