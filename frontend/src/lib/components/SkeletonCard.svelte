<script lang="ts">
	export let rows = 3;
</script>

<div class="skeleton-card">
	<div class="skeleton block"></div>
	{#each Array(rows) as _}
		<div class="skeleton line"></div>
	{/each}
</div>
