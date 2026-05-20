<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { CheckCircle2, Download, ListMusic, ListPlus, Loader2, MoreHorizontal, Share2, UserRound } from 'lucide-svelte';
	import { api } from '$lib/api';
	import { queueTrackAtTop, type QueueTrack } from '$lib/player';
	import type { PlaylistSummary } from '$lib/types';

	export let track: QueueTrack | null | undefined;
	export let artistHref: string | null = null;
	export let onDownload: (() => void | Promise<void>) | null = null;
	export let downloadDisabled = false;
	export let downloaded = false;
	export let downloading = false;

	let open = false;
	let pickingPlaylist = false;
	let playlists: PlaylistSummary[] = [];
	let playlistsLoading = false;
	let playlistError = '';
	let addingPlaylistId: string | null = null;
	let shareStatus = '';
	let root: HTMLDivElement | null = null;

	$: trackTitle = track?.title ?? 'track';
	$: canDownload = Boolean(track && onDownload) && !downloaded && !downloading && !downloadDisabled;

	onMount(() => {
		document.addEventListener('pointerdown', closeFromOutside);
	});

	onDestroy(() => {
		document.removeEventListener('pointerdown', closeFromOutside);
	});

	function closeFromOutside(event: PointerEvent) {
		if (!open || !root || root.contains(event.target as Node)) return;
		closeMenu();
	}

	function toggleMenu() {
		if (!track) return;
		open = !open;
		if (!open) resetNestedState();
	}

	function closeMenu() {
		open = false;
		resetNestedState();
	}

	function resetNestedState() {
		pickingPlaylist = false;
		playlistError = '';
		shareStatus = '';
	}

	async function showPlaylistPicker() {
		pickingPlaylist = !pickingPlaylist;
		playlistError = '';
		shareStatus = '';
		if (!pickingPlaylist || playlists.length || playlistsLoading) return;

		playlistsLoading = true;
		try {
			playlists = await api.playlists();
		} catch (error) {
			playlistError = error instanceof Error ? error.message : 'Unable to load playlists';
		} finally {
			playlistsLoading = false;
		}
	}

	async function addToPlaylist(playlist: PlaylistSummary) {
		if (!track) return;
		addingPlaylistId = playlist.id;
		playlistError = '';
		try {
			await api.addTrackToPlaylist(playlist.id, track.id);
			closeMenu();
		} catch (error) {
			playlistError = error instanceof Error ? error.message : 'Unable to add track';
		} finally {
			addingPlaylistId = null;
		}
	}

	function addToQueue() {
		if (!track) return;
		queueTrackAtTop(track);
		closeMenu();
	}

	function shareUrl() {
		if (!track) return '';
		if (typeof window === 'undefined') return '';
		if (track.albumId) return `${window.location.origin}/albums/${track.albumId}?track=${track.id}`;
		return window.location.href;
	}

	async function shareTrack() {
		if (!track) return;
		const url = shareUrl();
		const text = `${track.title} - ${track.artist}`;
		shareStatus = '';
		try {
			if (navigator.share) {
				await navigator.share({ title: track.title, text, url });
				closeMenu();
				return;
			}
			await navigator.clipboard.writeText(url);
			shareStatus = 'Link copied';
		} catch (error) {
			if (error instanceof DOMException && error.name === 'AbortError') return;
			shareStatus = 'Unable to share';
		}
	}

	async function downloadTrack() {
		if (!canDownload || !onDownload) return;
		await onDownload();
		closeMenu();
	}
</script>

<div class="track-actions" class:open bind:this={root}>
	<button
		class="icon-button track-actions-trigger"
		type="button"
		aria-label={`Track actions for ${trackTitle}`}
		aria-haspopup="menu"
		aria-expanded={open}
		disabled={!track}
		on:click|stopPropagation={toggleMenu}
	>
		<MoreHorizontal size={18} />
	</button>

	{#if open}
		<div class="track-actions-menu" role="menu">
			<button type="button" role="menuitem" on:click|stopPropagation={showPlaylistPicker}>
				<ListMusic size={16} />
				<span>Add to playlist</span>
			</button>

			{#if pickingPlaylist}
				<div class="playlist-picker">
					{#if playlistsLoading}
						<span class="menu-status"><Loader2 size={14} /> Loading playlists</span>
					{:else if playlistError}
						<span class="menu-status error">{playlistError}</span>
					{:else if playlists.length}
						{#each playlists as playlist}
							<button type="button" on:click|stopPropagation={() => addToPlaylist(playlist)} disabled={addingPlaylistId !== null}>
								{#if addingPlaylistId === playlist.id}<Loader2 size={14} />{:else}<ListMusic size={14} />{/if}
								<span>{playlist.name}</span>
							</button>
						{/each}
					{:else}
						<span class="menu-status">No playlists found</span>
					{/if}
				</div>
			{/if}

			<button type="button" role="menuitem" on:click|stopPropagation={addToQueue}>
				<ListPlus size={16} />
				<span>Add to queue</span>
			</button>

			{#if artistHref}
				<a role="menuitem" href={artistHref} on:click|stopPropagation={closeMenu}>
					<UserRound size={16} />
					<span>Go to artist</span>
				</a>
			{:else}
				<button type="button" role="menuitem" disabled>
					<UserRound size={16} />
					<span>Go to artist</span>
				</button>
			{/if}

			<button type="button" role="menuitem" on:click|stopPropagation={shareTrack}>
				<Share2 size={16} />
				<span>Share</span>
			</button>
			{#if shareStatus}<span class="menu-status">{shareStatus}</span>{/if}

			<button type="button" role="menuitem" on:click|stopPropagation={downloadTrack} disabled={!canDownload}>
				{#if downloading}<Loader2 size={16} />{:else if downloaded}<CheckCircle2 size={16} />{:else}<Download size={16} />{/if}
				<span>{downloaded ? 'Downloaded' : 'Download'}</span>
			</button>
		</div>
	{/if}
</div>
