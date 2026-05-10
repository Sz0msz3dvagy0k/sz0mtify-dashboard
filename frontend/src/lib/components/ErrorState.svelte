<script lang="ts">
	export let message = 'Request failed';
	export let retry: (() => void) | null = null;
</script>

<div class="error-state">
	<strong>{message}</strong>
	{#if retry}
		<button class="button ghost" on:click={retry}>Retry</button>
	{/if}
</div>
