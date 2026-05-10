<script lang="ts">
	import { formatDuration, formatNumber } from '$lib/format';
	export let title = '';
	export let detail = '';
	export let duration: number | null = null;
	export let count: number | null = null;
</script>

<div class="track-row">
	<div>
		<strong>{title}</strong>
		{#if detail}<span>{detail}</span>{/if}
	</div>
	<em>{count !== null ? formatNumber(count) : formatDuration(duration)}</em>
</div>
