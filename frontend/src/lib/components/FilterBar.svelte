<script lang="ts">
	export let value = '';
	export let placeholder = 'Filter';
</script>

<label class="filter-bar">
	<input bind:value {placeholder} />
</label>
