<script lang="ts">
	export let value = 18;
	export let total = 0;
	export let shown = 0;

	const options = [18, 36, 72];
</script>

<div class="items-per-page">
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
