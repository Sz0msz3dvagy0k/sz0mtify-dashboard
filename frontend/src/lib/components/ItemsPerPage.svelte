<script lang="ts">
	export let value = 18;
	export let page = 1;
	export let total = 0;
	export let pageTotal = total;
	export let shown = 0;

	const options = [18, 36, 72];

	$: pageCount = Math.max(1, Math.ceil(pageTotal / value));
	$: if (page > pageCount) page = pageCount;
	$: if (page < 1) page = 1;
</script>

<div class="items-per-page">
	<div class="pager-controls">
		<button class="icon-button" aria-label="Previous page" disabled={page <= 1} on:click={() => (page -= 1)}>‹</button>
		<span>Page {page} of {pageCount}</span>
		<button class="icon-button" aria-label="Next page" disabled={page >= pageCount} on:click={() => (page += 1)}>›</button>
	</div>
	<span>{shown} of {total}</span>
	<label>
		Items per page
		<select bind:value>
			{#each options as option}
				<option value={option}>{option}</option>
			{/each}
		</select>
	</label>
</div>
