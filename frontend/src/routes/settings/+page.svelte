<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { SyncStatus } from '$lib/types';
	import DataTable from '$lib/components/DataTable.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { apiBase } from '$lib/format';

	let health: { ok: boolean; status: string } | null = null;
	let status: SyncStatus = [];
	let settings: [string, string][] = [];
	let message = '';
	let error = '';
	let busy = '';

	async function load() {
		error = '';
		try {
			[health, status, settings] = await Promise.all([api.health(), api.syncStatus(), api.settings()]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load settings';
		}
	}

	async function run(label: string, fn: () => Promise<unknown>) {
		busy = label;
		message = '';
		try {
			await fn();
			message = `${label} started`;
			await load();
		} catch (e) {
			error = e instanceof Error ? e.message : `${label} failed`;
		} finally {
			busy = '';
		}
	}
	onMount(load);
	$: safeSettings = settings.map(([key, value]) => [key, /key|password|token|secret/i.test(key) ? '••••••••' : value]);
</script>

{#if error}<ErrorState message={error} retry={load} />{/if}
<section class="metric-grid">
	<StatCard label="Backend" value={health?.status ?? 'unknown'} meta={apiBase()} />
	<StatCard label="Sync Rows" value={status.length} />
	<StatCard label="Stored Settings" value={settings.length} meta="secrets masked" />
</section>
<div class="toolbar">
	<button class="button" disabled={!!busy} on:click={() => run('Subsonic sync', api.syncSubsonic)}>Subsonic Sync</button>
	<button class="button" disabled={!!busy} on:click={() => run('Last.fm sync', api.syncLastfm)}>Last.fm Sync</button>
	<button class="button" disabled={!!busy} on:click={() => run('Full sync', api.syncAll)}>Full Sync</button>
	<button class="button ghost" disabled={!!busy} on:click={() => run('Discovery refresh', () => api.refreshDiscovery(10))}>Discovery Refresh</button>
	{#if message}<span class="muted">{message}</span>{/if}
</div>
<DataTable columns={['ID', 'Source', 'Last Sync', 'Status', 'Error']} rows={status} />
<DataTable columns={['Setting', 'Value']} rows={safeSettings} />
