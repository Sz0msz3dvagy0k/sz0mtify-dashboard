<script lang="ts">
	export let playing = false;
	export let highlighted = false;
	export let onSelect: () => void = () => {};
</script>

<tr class:playing-row={playing} class:highlight-row={highlighted} on:click={onSelect}>
	<slot />
</tr>
