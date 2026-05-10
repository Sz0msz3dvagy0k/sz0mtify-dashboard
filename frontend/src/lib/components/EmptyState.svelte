<script lang="ts">
	export let title = 'Nothing here';
	export let detail = '';
</script>

<div class="empty-state">
	<div class="empty-mark"></div>
	<strong>{title}</strong>
	{#if detail}<p>{detail}</p>{/if}
</div>
