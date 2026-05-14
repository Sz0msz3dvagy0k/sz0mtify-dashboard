<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { SyncStatus } from '$lib/types';
	import DataTable from '$lib/components/DataTable.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { apiBase } from '$lib/format';
	import { initNetworkStatus, networkStatus } from '$lib/mobileNetwork';

	let health: { ok: boolean; status: string } | null = null;
	let status: SyncStatus = [];
	let settings: [string, string][] = [];
	let message = '';
	let error = '';
	let busy = '';
	let transcodeMode = 'never';
	let transcodeQuality = '192';

	async function load() {
		error = '';
		try {
			[health, status, settings] = await Promise.all([api.health(), api.syncStatus(), api.settings()]);
			const settingsMap = new Map(settings);
			transcodeMode = settingsMap.get('stream_transcode_mode') ?? 'never';
			transcodeQuality = settingsMap.get('stream_transcode_quality') ?? '192';
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load settings';
		}
	}

	async function saveTranscoding() {
		busy = 'Transcoding settings';
		message = '';
		error = '';
		try {
			await api.saveSettings({
				stream_transcode_mode: transcodeMode,
				stream_transcode_quality: transcodeQuality
			});
			message = 'Transcoding settings saved';
			await load();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to save transcoding settings';
		} finally {
			busy = '';
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
	onMount(() => {
		void initNetworkStatus();
		void load();
	});
	$: safeSettings = settings.map(([key, value]) => [key, /key|password|token|secret/i.test(key) ? '••••••••' : value]);
</script>

{#if error}<ErrorState message={error} retry={load} />{/if}
<section class="metric-grid">
	<StatCard label="Backend" value={health?.status ?? 'unknown'} meta={apiBase()} />
	<StatCard label="Sync Rows" value={status.length} />
	<StatCard label="Stored Settings" value={settings.length} meta="secrets masked" />
	<StatCard label="Network" value={$networkStatus.connectionType} meta={$networkStatus.connected ? 'connected' : 'offline'} />
</section>
<div class="toolbar">
	<button class="button" disabled={!!busy} on:click={() => run('Subsonic sync', api.syncSubsonic)}>Subsonic Sync</button>
	<button class="button" disabled={!!busy} on:click={() => run('Last.fm sync', api.syncLastfm)}>Last.fm Sync</button>
	<button class="button" disabled={!!busy} on:click={() => run('Full sync', api.syncAll)}>Full Sync</button>
	<button class="button ghost" disabled={!!busy} on:click={() => run('Discovery refresh', () => api.refreshDiscovery(10))}>Discovery Refresh</button>
	{#if message}<span class="muted">{message}</span>{/if}
</div>
<section class="settings-panel">
	<div>
		<p class="eyebrow">Playback</p>
		<h2>Transcoding</h2>
	</div>
	<label>
		<span>When to transcode</span>
		<select bind:value={transcodeMode}>
			<option value="never">Never</option>
			<option value="cellular">Mobile internet only</option>
			<option value="always">Always</option>
		</select>
	</label>
	<label>
		<span>Quality</span>
		<select bind:value={transcodeQuality}>
			<option value="96">96 kbps</option>
			<option value="128">128 kbps</option>
			<option value="192">192 kbps</option>
			<option value="256">256 kbps</option>
			<option value="320">320 kbps</option>
		</select>
	</label>
	<button class="button" disabled={!!busy} on:click={saveTranscoding}>Save Playback</button>
</section>
<DataTable columns={['ID', 'Source', 'Last Sync', 'Status', 'Error']} rows={status} />
<DataTable columns={['Setting', 'Value']} rows={safeSettings} />
