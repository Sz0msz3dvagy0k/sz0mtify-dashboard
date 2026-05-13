<script lang="ts">
	export let columns: string[] = [];
	export let rows: (string | number | null | undefined)[][] = [];
	export let cellLinks: (string | null | undefined)[][] = [];
</script>

<div class="table-wrap">
	<table>
		<thead>
			<tr>{#each columns as column}<th>{column}</th>{/each}</tr>
		</thead>
		<tbody>
			{#each rows as row, rowIndex}
				<tr>
					{#each row as cell, index}
						<td>
							{#if cellLinks[rowIndex]?.[index]}
								<a class="table-link" href={cellLinks[rowIndex]?.[index]}>{cell ?? '—'}</a>
							{:else}
								{cell ?? '—'}
							{/if}
						</td>
					{/each}
				</tr>
			{/each}
		</tbody>
	</table>
</div>
