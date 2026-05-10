<script lang="ts">
	export let columns: string[] = [];
	export let rows: (string | number | null | undefined)[][] = [];
</script>

<div class="table-wrap">
	<table>
		<thead>
			<tr>{#each columns as column}<th>{column}</th>{/each}</tr>
		</thead>
		<tbody>
			{#each rows as row}
				<tr>{#each row as cell}<td>{cell ?? '—'}</td>{/each}</tr>
			{/each}
		</tbody>
	</table>
</div>
