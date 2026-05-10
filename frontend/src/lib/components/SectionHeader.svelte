<script lang="ts">
	export let title = '';
	export let eyebrow = '';
	export let action = '';
	export let href = '';
</script>

<header class="section-header">
	<div>
		{#if eyebrow}<span>{eyebrow}</span>{/if}
		<h2>{title}</h2>
	</div>
	{#if action && href}
		<a class="text-link" {href}>{action}</a>
	{/if}
</header>
