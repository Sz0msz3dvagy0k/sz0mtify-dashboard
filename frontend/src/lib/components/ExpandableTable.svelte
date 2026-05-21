<script lang="ts">
	import { ChevronDown, ChevronUp } from 'lucide-svelte';

	type ExpandableRow = {
		id: string | number;
		title: string;
		href?: string | null;
		titleClass?: string;
		details: [string, string | number | null | undefined][];
	};

	export let rows: ExpandableRow[] = [];
	export let title = '';

	let openRows = new Set<string | number>();

	function toggle(id: string | number) {
		openRows = new Set(openRows);
		if (openRows.has(id)) {
			openRows.delete(id);
		} else {
			openRows.add(id);
		}
	}
</script>

<section class="expandable-table">
	{#if title}<header>{title}</header>{/if}
	{#each rows as row}
		<article class:open={openRows.has(row.id)}>
			<div class="expandable-table-main">
				{#if row.href}
					<a class={`table-link ${row.titleClass ?? ''}`.trim()} href={row.href}>{row.title}</a>
				{:else}
					<strong class={row.titleClass ?? ''}>{row.title}</strong>
				{/if}
				<button class="icon-button expand-toggle" aria-label={openRows.has(row.id) ? 'Hide details' : 'Show details'} on:click={() => toggle(row.id)}>
					{#if openRows.has(row.id)}
						<ChevronUp size={18} strokeWidth={1.8} />
					{:else}
						<ChevronDown size={18} strokeWidth={1.8} />
					{/if}
				</button>
			</div>
			{#if openRows.has(row.id)}
				<div class="expandable-table-details">
					{#each row.details as [label, value]}
						<div>
							<span>{label}</span>
							<strong>{value ?? '—'}</strong>
						</div>
					{/each}
				</div>
			{/if}
		</article>
	{/each}
</section>
