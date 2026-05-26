<script lang="ts">
	import { onMount } from 'svelte';
	import { ChevronDown, ChevronUp, Loader2, MoreHorizontal, Trash2 } from 'lucide-svelte';
	import { api } from '$lib/api';
	import { clearAuthSession } from '$lib/auth';
	import type { ActiveSession, SyncStatus } from '$lib/types';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import { apiBase } from '$lib/format';
	import { initNetworkStatus, networkStatus } from '$lib/mobileNetwork';
	import { themeSettings, updateTheme, type ThemeMode, type ThemePalette } from '$lib/theme';

	const themeModes: { value: ThemeMode; label: string }[] = [
		{ value: 'dark', label: 'Dark' },
		{ value: 'light', label: 'Light' }
	];
	const themePalettes: { value: ThemePalette; label: string }[] = [
		{ value: 'monochrome', label: 'Mono' },
		{ value: 'ocean', label: 'Ocean' },
		{ value: 'forest', label: 'Forest' },
		{ value: 'rose', label: 'Rose' }
	];

	let health: { ok: boolean; status: string } | null = null;
	let status: SyncStatus = [];
	let settings: [string, string][] = [];
	let sessions: ActiveSession[] = [];
	let message = '';
	let error = '';
	let busy = '';
	let openSessionMenuId: string | null = null;
	let expandedSessionId: string | null = null;
	let deletingSessionId: string | null = null;
	let sessionTableRoot: HTMLDivElement | null = null;
	let transcodeMode = 'never';
	let transcodeQuality = '192';

	async function load() {
		error = '';
		try {
			[health, status, settings, sessions] = await Promise.all([
				api.health(),
				api.syncStatus(),
				api.settings(),
				api.activeSessions()
			]);
			const settingsMap = new Map(settings);
			transcodeMode = settingsMap.get('stream_transcode_mode') ?? 'never';
			transcodeQuality = settingsMap.get('stream_transcode_quality') ?? '192';
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to load settings';
		}
	}

	function formatSessionTime(value: number) {
		if (!value) return '—';
		return new Intl.DateTimeFormat(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		}).format(new Date(value * 1000));
	}

	function displayCell(value: string | number | null | undefined) {
		return value === null || value === undefined || value === '' ? '—' : String(value);
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
		error = '';
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

	function closeSessionMenuFromOutside(event: PointerEvent) {
		if (!openSessionMenuId || !sessionTableRoot || sessionTableRoot.contains(event.target as Node)) return;
		openSessionMenuId = null;
	}

	function toggleSessionMenu(sessionId: string) {
		openSessionMenuId = openSessionMenuId === sessionId ? null : sessionId;
	}

	function toggleSessionDetails(sessionId: string) {
		expandedSessionId = expandedSessionId === sessionId ? null : sessionId;
	}

	async function deleteSession(session: ActiveSession) {
		deletingSessionId = session.session_id;
		error = '';
		message = '';
		try {
			await api.deleteSession(session.session_id);
			openSessionMenuId = null;

			if (session.current) {
				clearAuthSession();
				window.location.reload();
				return;
			}

			message = 'Session deleted';
			await load();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to delete session';
		} finally {
			deletingSessionId = null;
		}
	}

	onMount(() => {
		document.addEventListener('pointerdown', closeSessionMenuFromOutside);
		void initNetworkStatus();
		void load();
		return () => document.removeEventListener('pointerdown', closeSessionMenuFromOutside);
	});
	$: safeSettings = settings.map(([key, value]) => [key, /key|password|token|secret/i.test(key) ? '••••••••' : value]);
</script>

{#if error}<ErrorState message={error} retry={load} />{/if}
<section class="metric-grid">
	<StatCard label="Backend" value={health?.status ?? 'unknown'} meta={apiBase()} />
	<StatCard label="Sync Rows" value={status.length} />
	<StatCard label="Stored Settings" value={settings.length} meta="secrets masked" />
	<StatCard label="Active Sessions" value={sessions.length} />
	<StatCard label="Network" value={$networkStatus.connectionType} meta={$networkStatus.connected ? 'connected' : 'offline'} />
</section>
<div class="toolbar">
	<button class="button" disabled={!!busy} on:click={() => run('Subsonic sync', api.syncSubsonic)}>Subsonic Sync</button>
	<button class="button" disabled={!!busy} on:click={() => run('Last.fm sync', api.syncLastfm)}>Last.fm Sync</button>
	<button class="button" disabled={!!busy} on:click={() => run('Full sync', api.syncAll)}>Full Sync</button>
	<button class="button ghost" disabled={!!busy} on:click={() => run('Discovery refresh', () => api.refreshDiscovery(10))}>Discovery Refresh</button>
	{#if message}<span class="muted">{message}</span>{/if}
</div>
<section class="settings-panel theme-settings-panel">
	<div>
		<p class="eyebrow">Appearance</p>
		<h2>Theme</h2>
	</div>
	<div class="theme-control">
		<span>Mode</span>
		<div class="segmented-control" role="group" aria-label="Theme mode">
			{#each themeModes as mode}
				<button class:active={$themeSettings.mode === mode.value} type="button" on:click={() => updateTheme({ mode: mode.value })}>
					{mode.label}
				</button>
			{/each}
		</div>
	</div>
	<div class="theme-control palette-control">
		<span>Palette</span>
		<div class="palette-options" role="group" aria-label="Theme palette">
			{#each themePalettes as palette}
				<button class:active={$themeSettings.palette === palette.value} class={`palette-option ${palette.value}`} type="button" on:click={() => updateTheme({ palette: palette.value })}>
					<span class="palette-swatch"></span>
					<span>{palette.label}</span>
				</button>
			{/each}
		</div>
	</div>
</section>
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
<div class="table-wrap session-table-wrap" bind:this={sessionTableRoot}>
	<table class="session-table">
		<thead>
			<tr>
				<th>Session</th>
				<th>User</th>
				<th>Last Seen</th>
				<th>Created</th>
				<th>Expires</th>
				<th aria-label="Actions"></th>
			</tr>
		</thead>
		<tbody>
			{#each sessions as session}
				<tr>
					<td>{session.current ? `${session.session_id} · current` : session.session_id}</td>
					<td>{session.username}</td>
					<td>{formatSessionTime(session.last_seen_at)}</td>
					<td>{formatSessionTime(session.created_at)}</td>
					<td>{formatSessionTime(session.expires_at)}</td>
					<td>
						<div class="track-actions" class:open={openSessionMenuId === session.session_id}>
							<button
								class="icon-button track-actions-trigger"
								type="button"
								aria-label={`Session actions for ${session.session_id}`}
								aria-haspopup="menu"
								aria-expanded={openSessionMenuId === session.session_id}
								disabled={deletingSessionId !== null}
								on:click|stopPropagation={() => toggleSessionMenu(session.session_id)}
							>
								{#if deletingSessionId === session.session_id}<Loader2 size={18} />{:else}<MoreHorizontal size={18} />{/if}
							</button>

							{#if openSessionMenuId === session.session_id}
								<div class="track-actions-menu" role="menu">
									<button
										class="danger-menu-item"
										type="button"
										role="menuitem"
										disabled={deletingSessionId !== null}
										on:click|stopPropagation={() => deleteSession(session)}
									>
										<Trash2 size={16} />
										<span>Delete session</span>
									</button>
								</div>
							{/if}
						</div>
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
	<div class="mobile-session-list">
		{#each sessions as session}
			<article class:open={expandedSessionId === session.session_id}>
				<div class="mobile-session-main">
					<div>
						<strong>{session.session_id}</strong>
						{#if session.current}<span>Current</span>{/if}
					</div>
					<div class="mobile-session-seen">
						<span>{formatSessionTime(session.last_seen_at)}</span>
						<button
							class="icon-button expand-toggle"
							type="button"
							aria-label={`${expandedSessionId === session.session_id ? 'Hide' : 'Show'} details for ${session.session_id}`}
							aria-expanded={expandedSessionId === session.session_id}
							on:click={() => toggleSessionDetails(session.session_id)}
						>
							{#if expandedSessionId === session.session_id}<ChevronUp size={18} />{:else}<ChevronDown size={18} />{/if}
						</button>
					</div>
				</div>
				{#if expandedSessionId === session.session_id}
					<div class="mobile-session-details">
						<div>
							<span>User</span>
							<strong>{session.username}</strong>
						</div>
						<div>
							<span>Created</span>
							<strong>{formatSessionTime(session.created_at)}</strong>
						</div>
						<div>
							<span>Expires</span>
							<strong>{formatSessionTime(session.expires_at)}</strong>
						</div>
						<button
							class="mobile-session-delete"
							type="button"
							disabled={deletingSessionId !== null}
							on:click={() => deleteSession(session)}
						>
							{#if deletingSessionId === session.session_id}<Loader2 size={16} />{:else}<Trash2 size={16} />{/if}
							<span>Delete session</span>
						</button>
					</div>
				{/if}
			</article>
		{/each}
	</div>
</div>
<div class="table-wrap sync-status-wrap">
	<table class="sync-status-table">
		<thead>
			<tr>
				<th>ID</th>
				<th>Source</th>
				<th>Last Sync</th>
				<th>Status</th>
				<th>Error</th>
			</tr>
		</thead>
		<tbody>
			{#each status as row}
				<tr>
					<td>{row[0]}</td>
					<td>{displayCell(row[1])}</td>
					<td>{displayCell(row[2])}</td>
					<td>{displayCell(row[3])}</td>
					<td>{displayCell(row[4])}</td>
				</tr>
			{/each}
		</tbody>
	</table>
	<div class="mobile-sync-status-list">
		{#each status as row}
			<article>
				<div class="mobile-sync-status-main">
					<strong>{displayCell(row[1])}</strong>
					<span>{displayCell(row[3])}</span>
				</div>
				<div class="mobile-sync-status-details">
					<div>
						<span>ID</span>
						<strong>{row[0]}</strong>
					</div>
					<div>
						<span>Last Sync</span>
						<strong>{displayCell(row[2])}</strong>
					</div>
					<div>
						<span>Error</span>
						<strong>{displayCell(row[4])}</strong>
					</div>
				</div>
			</article>
		{/each}
	</div>
</div>
<div class="table-wrap settings-values-wrap">
	<table class="settings-values-table">
		<thead>
			<tr>
				<th>Setting</th>
				<th>Value</th>
			</tr>
		</thead>
		<tbody>
			{#each safeSettings as row}
				<tr>
					<td>{row[0]}</td>
					<td>{displayCell(row[1])}</td>
				</tr>
			{/each}
		</tbody>
	</table>
	<div class="mobile-settings-values-list">
		{#each safeSettings as row}
			<article>
				<div>
					<span>Setting</span>
					<strong>{row[0]}</strong>
				</div>
				<div>
					<span>Value</span>
					<strong>{displayCell(row[1])}</strong>
				</div>
			</article>
		{/each}
	</div>
</div>
