<script lang="ts">
	export let label = '';
	export let value: string | number = '';
	export let meta = '';
</script>

<article class="stat-card">
	<p>{label}</p>
	<strong>{value}</strong>
	{#if meta}<span>{meta}</span>{/if}
</article>
