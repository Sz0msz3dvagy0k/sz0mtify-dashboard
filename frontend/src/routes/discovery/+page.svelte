<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { DiscoveryList, DiscoveryRefresh } from '$lib/types';
	import DiscoveryCard from '$lib/components/DiscoveryCard.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ItemsPerPage from '$lib/components/ItemsPerPage.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { formatNumber } from '$lib/format';

	let newReleases: DiscoveryList;
	let missing: DiscoveryList;
	let similar: DiscoveryList;
	let refreshResult: DiscoveryRefresh | null = null;
	let loading = true;
	let refreshing = false;
	let error = '';
	let includeOwned = false;
	let itemsPerPage = 18;
	let pageIndex = 1;

	async function load() {
		loading = true;
		error = '';
		const owned = includeOwned ? '&include_owned=true' : '';
		try {
			[newReleases, missing, similar] = await Promise.all([
				api.newReleases(`limit=72${owned}`),
				api.missingAlbums(`limit=72${owned}`),
				api.similarArtists(`limit=72${owned}`)
			]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load discovery';
		} finally {
			loading = false;
		}
	}

	async function refresh() {
		refreshing = true;
		try {
			refreshResult = await api.refreshDiscovery(10);
			await load();
		} finally {
			refreshing = false;
		}
	}

	onMount(load);
	$: pageStart = (pageIndex - 1) * itemsPerPage;
	$: pagedNewReleases = newReleases?.items.slice(pageStart, pageStart + itemsPerPage) ?? [];
	$: pagedMissing = missing?.items.slice(pageStart, pageStart + itemsPerPage) ?? [];
	$: pagedSimilar = similar?.items.slice(pageStart, pageStart + itemsPerPage) ?? [];
	$: shownDiscoveryItems =
		pagedNewReleases.length +
		pagedMissing.length +
		pagedSimilar.length;
	$: totalDiscoveryItems = (newReleases?.total ?? 0) + (missing?.total ?? 0) + (similar?.total ?? 0);
	$: pageTotal = Math.max(newReleases?.items.length ?? 0, missing?.items.length ?? 0, similar?.items.length ?? 0);
	$: if (pageIndex > Math.max(1, Math.ceil(pageTotal / itemsPerPage))) pageIndex = 1;
</script>

{#if loading}
	<div class="skeleton-card"></div>
{:else if error}
	<ErrorState message={error} retry={load} />
{:else}
	<section class="metric-grid">
		<StatCard label="New Radar" value={formatNumber(newReleases.total)} meta="albums and tracks" />
		<StatCard label="Missing Albums" value={formatNumber(missing.total)} />
		<StatCard label="Similar Artists" value={formatNumber(similar.total)} />
	</section>
	<div class="toolbar">
		<button class="button" disabled={refreshing} on:click={refresh}>{refreshing ? 'Refreshing…' : 'Refresh Discovery'}</button>
		<label class="check"><input type="checkbox" bind:checked={includeOwned} on:change={load} /> Include owned</label>
		{#if refreshResult}<span class="muted">Created {refreshResult.created_count}, updated {refreshResult.updated_count}</span>{/if}
	</div>
	<SectionHeader title="New From Your Artists" eyebrow="Last.fm" />
	<div class="discovery-grid">{#each pagedNewReleases as item}<DiscoveryCard {item} />{/each}</div>
	{#if !newReleases.items.length}<EmptyState title="No discovery rows yet" detail="Run refresh to generate candidates." />{/if}
	<SectionHeader title="Missing Albums" eyebrow="not owned by default" />
	<div class="discovery-grid">{#each pagedMissing as item}<DiscoveryCard {item} />{/each}</div>
	<SectionHeader title="Similar Artists" eyebrow="outside library" />
	<div class="discovery-grid">{#each pagedSimilar as item}<DiscoveryCard {item} />{/each}</div>
	<ItemsPerPage bind:value={itemsPerPage} bind:page={pageIndex} total={totalDiscoveryItems} pageTotal={pageTotal} shown={shownDiscoveryItems} />
{/if}
