<script lang="ts">
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import {
		ChevronsLeft,
		ChevronsRight,
		ListMusic,
		MessageSquareText,
		Pause,
		Play,
		Volume2
	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import ImageWithFallback from './ImageWithFallback.svelte';
	import { loadCachedImage } from '$lib/imageCache';
	import { formatDuration } from '$lib/format';
	import {
		playIndex,
		playNext,
		playPrevious,
		player,
		queueTrackImage,
		recordSongHistory,
		setPlaying,
		setTime,
		setVolume,
		streamUrl,
		togglePlay,
		toggleQueue
	} from '$lib/player';

	let audio: HTMLAudioElement;
	let lastTrackId: number | null = null;
	let registeredTrackId: number | null = null;
	let pendingAutoplayTrackId: number | null = null;
	let expanded = false;
	let playerTouchStartY = 0;
	let playerTouchStartX = 0;
	let playerTouchStartTime = 0;
	let playerDragOffset = 0;
	let playerDragMode: 'opening' | 'closing' | null = null;
	let playerPointerId: number | null = null;
	let suppressNextPlayerClick = false;
	let mediaSessionTrackId: number | null = null;
	let streamRequestId = 0;
	$: currentTrack = $player.queue[$player.currentIndex] ?? null;
	$: progress = $player.duration > 0 ? ($player.currentTime / $player.duration) * 100 : 0;
	$: visualExpanded = expanded || playerDragMode !== null;
	$: playerDragStyle = playerDragMode ? `transform: translate3d(0, ${Math.round(playerDragOffset)}px, 0);` : '';

	$: if (audio && currentTrack && currentTrack.id !== lastTrackId) {
		lastTrackId = currentTrack.id;
		void loadTrackStream(currentTrack.id, ++streamRequestId);
	}

	$: if (audio && Math.abs(audio.volume - $player.volume) > 0.01) {
		audio.volume = $player.volume;
	}

	$: if (audio && currentTrack && $player.isPlaying && audio.paused) {
		void playAudio(currentTrack.id);
	}

	$: if (audio && !$player.isPlaying && !audio.paused) {
		audio.pause();
	}

	$: if (currentTrack && currentTrack.id !== mediaSessionTrackId) {
		mediaSessionTrackId = currentTrack.id;
		void updateMediaSessionMetadata(currentTrack);
	}

	$: updateMediaSessionPlaybackState();
	$: updateMediaSessionPositionState();

	function seek(event: Event) {
		const value = Number((event.target as HTMLInputElement).value);
		if (!audio || !$player.duration) return;
		audio.currentTime = (value / 100) * $player.duration;
	}

	function handleGlobalKeydown(event: KeyboardEvent) {
		if (event.code !== 'Space' || event.repeat || !currentTrack || isEditingTarget(event.target)) return;
		event.preventDefault();
		togglePlay();
	}

	async function updateMediaSessionMetadata(track: NonNullable<typeof currentTrack>) {
		if (!browser || !('mediaSession' in navigator) || !('MediaMetadata' in window)) return;

		const artwork = await mediaSessionArtwork(track);
		if (currentTrack?.id !== track.id) return;

		navigator.mediaSession.metadata = new MediaMetadata({
			title: track.title,
			artist: track.artist,
			album: track.album,
			artwork
		});
	}

	async function mediaSessionArtwork(track: NonNullable<typeof currentTrack>): Promise<MediaImage[]> {
		const imageUrl = queueTrackImage(track);
		if (!imageUrl) return [];

		try {
			const src = await loadCachedImage(imageUrl);
			return [
				{ src, sizes: '96x96', type: 'image/png' },
				{ src, sizes: '256x256', type: 'image/png' },
				{ src, sizes: '512x512', type: 'image/png' }
			];
		} catch (error) {
			console.warn('Unable to load lock-screen artwork', error);
			return [];
		}
	}

	function updateMediaSessionPlaybackState() {
		if (!browser || !('mediaSession' in navigator)) return;
		navigator.mediaSession.playbackState = currentTrack
			? $player.isPlaying
				? 'playing'
				: 'paused'
			: 'none';
	}

	function updateMediaSessionPositionState() {
		if (!browser || !('mediaSession' in navigator) || !navigator.mediaSession.setPositionState) return;
		const duration = $player.duration || currentTrack?.duration || 0;
		if (!currentTrack || !Number.isFinite(duration) || duration <= 0) return;

		try {
			navigator.mediaSession.setPositionState({
				duration,
				playbackRate: audio?.playbackRate || 1,
				position: Math.min($player.currentTime, duration)
			});
		} catch {
			// Some WebKit builds reject position state until duration is fully known.
		}
	}

	function registerMediaSessionHandlers() {
		if (!browser || !('mediaSession' in navigator)) return;
		const handlers: Partial<Record<MediaSessionAction, MediaSessionActionHandler>> = {
			play: () => setPlaying(true),
			pause: () => setPlaying(false),
			previoustrack: playPrevious,
			nexttrack: playNext,
			seekbackward: playPrevious,
			seekforward: playNext,
			seekto: (details) => {
				if (!audio || typeof details.seekTime !== 'number') return;
				audio.currentTime = details.seekTime;
			}
		};

		for (const [action, handler] of Object.entries(handlers) as [MediaSessionAction, MediaSessionActionHandler][]) {
			try {
				navigator.mediaSession.setActionHandler(action, handler);
			} catch {
				// Ignore actions unsupported by the current browser/runtime.
			}
		}
	}

	function isEditingTarget(target: EventTarget | null) {
		if (!(target instanceof HTMLElement)) return false;
		if (target.isContentEditable) return true;
		return Boolean(target.closest('input, textarea, select, button, a, [role="button"], [role="slider"]'));
	}

	function isPlayerControlTarget(target: EventTarget | null) {
		if (!(target instanceof HTMLElement)) return false;
		if (target.isContentEditable) return true;
		return Boolean(target.closest('input, textarea, select, button, a, [role="slider"]'));
	}

	async function loadTrackStream(trackId: number, requestId: number) {
		try {
			console.debug('[player] loading stream', { trackId });
			const src = await streamUrl(trackId);
			if (requestId !== streamRequestId || currentTrack?.id !== trackId) return;
			audio.src = src;
			audio.load();
			if ($player.isPlaying) pendingAutoplayTrackId = trackId;
			if ($player.isPlaying) void playAudio(trackId);
		} catch (error) {
			console.warn('[player] stream load failed', { trackId, error });
			if (requestId === streamRequestId) setPlaying(false);
		}
	}

	async function playAudio(trackId: number) {
		try {
			console.debug('[player] play requested', { trackId, readyState: audio.readyState, networkState: audio.networkState });
			await audio.play();
			pendingAutoplayTrackId = null;
			console.debug('[player] playback started', { trackId });
			if (registeredTrackId !== trackId) {
				await api
					.nowPlaying(trackId)
					.then(() => {
						registeredTrackId = trackId;
					})
					.catch((error) => {
						console.warn('Unable to register now playing', error);
					});
			}
			if (currentTrack?.id === trackId) recordSongHistory(currentTrack);
		} catch (error) {
			console.warn('[player] audio.play failed', {
				trackId,
				error,
				mediaError: describeMediaError(audio.error),
				networkState: audio.networkState,
				readyState: audio.readyState
			});
			if (currentTrack?.id === trackId && $player.isPlaying && audio.readyState < HTMLMediaElement.HAVE_FUTURE_DATA) {
				pendingAutoplayTrackId = trackId;
				return;
			}
			pendingAutoplayTrackId = null;
			setPlaying(false);
		}
	}

	function handleMediaReady(eventName: string) {
		setTime(audio.currentTime, audio.duration);
		logAudioEvent(eventName);
		if (pendingAutoplayTrackId && currentTrack?.id === pendingAutoplayTrackId && $player.isPlaying) {
			void playAudio(pendingAutoplayTrackId);
		}
	}

	function toggleExpanded(event: MouseEvent | KeyboardEvent) {
		if (suppressNextPlayerClick) {
			suppressNextPlayerClick = false;
			return;
		}
		if (isPlayerControlTarget(event.target)) return;
		expanded = true;
	}

	function handlePlayerPointerDown(event: PointerEvent) {
		if (event.pointerType === 'mouse' || isPlayerControlTarget(event.target)) return;
		const target = event.currentTarget;
		if (target instanceof HTMLElement) target.setPointerCapture(event.pointerId);
		playerPointerId = event.pointerId;
		playerTouchStartX = event.clientX;
		playerTouchStartY = event.clientY;
		playerTouchStartTime = performance.now();
		playerDragOffset = expanded ? 0 : closedPlayerOffset();
		playerDragMode = null;
	}

	function handlePlayerPointerMove(event: PointerEvent) {
		if (playerPointerId !== event.pointerId) return;
		const deltaX = event.clientX - playerTouchStartX;
		const deltaY = event.clientY - playerTouchStartY;
		if (Math.abs(deltaX) > 80 || Math.abs(deltaY) < 8) return;

		if (!playerDragMode) {
			if (!expanded && deltaY < 0) {
				playerDragMode = 'opening';
				suppressNextPlayerClick = true;
			} else if (expanded && deltaY > 0) {
				playerDragMode = 'closing';
				suppressNextPlayerClick = true;
			} else {
				return;
			}
		}

		event.preventDefault();
		if (playerDragMode === 'opening') {
			playerDragOffset = clamp(closedPlayerOffset() + deltaY, 0, closedPlayerOffset());
		} else {
			playerDragOffset = clamp(deltaY, 0, closedPlayerOffset());
		}
	}

	function handlePlayerPointerUp(event: PointerEvent) {
		if (playerPointerId !== event.pointerId) return;
		playerPointerId = null;
		if (!playerDragMode) {
			resetPlayerDrag();
			return;
		}
		const deltaY = event.clientY - playerTouchStartY;
		const elapsed = Math.max(1, performance.now() - playerTouchStartTime);
		const velocity = deltaY / elapsed;
		const closedOffset = closedPlayerOffset();
		const openDistance = closedOffset - playerDragOffset;
		const releaseDistance = Math.min(140, closedOffset * 0.22);

		expanded = playerDragMode === 'opening'
			? openDistance > releaseDistance || velocity < -0.25
			: !(playerDragOffset > releaseDistance || velocity > 0.25);
		playerDragMode = null;
		playerDragOffset = 0;
	}

	function resetPlayerDrag() {
		playerDragMode = null;
		playerDragOffset = 0;
		playerPointerId = null;
	}

	function closedPlayerOffset() {
		if (!browser) return 640;
		return Math.max(220, window.innerHeight - 92);
	}

	function clamp(value: number, min: number, max: number) {
		return Math.min(Math.max(value, min), max);
	}

	function logAudioEvent(eventName: string) {
		console.debug(`[player] audio ${eventName}`, {
			trackId: currentTrack?.id ?? null,
			currentSrc: redactStreamToken(audio.currentSrc),
			networkState: audio.networkState,
			readyState: audio.readyState,
			mediaError: describeMediaError(audio.error)
		});
	}

	function logAudioError() {
		console.warn('[player] audio element error', {
			trackId: currentTrack?.id ?? null,
			currentSrc: redactStreamToken(audio.currentSrc),
			networkState: audio.networkState,
			readyState: audio.readyState,
			mediaError: describeMediaError(audio.error)
		});
	}

	function describeMediaError(error: MediaError | null) {
		if (!error) return null;
		const names: Record<number, string> = {
			[MediaError.MEDIA_ERR_ABORTED]: 'MEDIA_ERR_ABORTED',
			[MediaError.MEDIA_ERR_NETWORK]: 'MEDIA_ERR_NETWORK',
			[MediaError.MEDIA_ERR_DECODE]: 'MEDIA_ERR_DECODE',
			[MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED]: 'MEDIA_ERR_SRC_NOT_SUPPORTED'
		};
		return {
			code: error.code,
			name: names[error.code] ?? 'MEDIA_ERR_UNKNOWN',
			message: error.message
		};
	}

	function redactStreamToken(value: string) {
		if (!value) return '';
		return value.replace(/([?&]stream_token=)[^&]+/, '$1[redacted]');
	}

	onMount(() => {
		if (audio) audio.volume = $player.volume;
		registerMediaSessionHandlers();
	});
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

<audio
	bind:this={audio}
	crossorigin="use-credentials"
	on:timeupdate={() => setTime(audio.currentTime, audio.duration)}
	on:loadedmetadata={() => handleMediaReady('loadedmetadata')}
	on:canplay={() => handleMediaReady('canplay')}
	on:stalled={() => logAudioEvent('stalled')}
	on:error={logAudioError}
	on:ended={playNext}
></audio>

{#if currentTrack}
	<div
		class="player-bar"
		class:expanded={visualExpanded}
		class:dragging={playerDragMode !== null}
		style={playerDragStyle}
		on:click={toggleExpanded}
		on:keydown={(event) => (event.key === 'Enter' || event.key === ' ') && toggleExpanded(event)}
		on:pointerdown={handlePlayerPointerDown}
		on:pointermove={handlePlayerPointerMove}
		on:pointerup={handlePlayerPointerUp}
		on:pointercancel={resetPlayerDrag}
		role="button"
		tabindex="0"
	>
		{#if visualExpanded}
			<button class="player-collapse" aria-label="Collapse player" on:click|stopPropagation={() => (expanded = false)}>×</button>
			<div class="player-expanded-art">
				<ImageWithFallback src={queueTrackImage(currentTrack)} alt={currentTrack.title} />
			</div>
			<div class="player-expanded-main">
				<div class="player-expanded-title">
					<strong>{currentTrack.title}</strong>
					<span>{currentTrack.artist} · {currentTrack.album}</span>
				</div>

				<div class="player-center">
					<div class="player-buttons">
						<button class="icon-button" aria-label="Previous track" on:click|stopPropagation={playPrevious} disabled={$player.currentIndex <= 0}>
							<ChevronsLeft size={18} />
						</button>
						<button class="icon-button primary" aria-label={$player.isPlaying ? 'Pause' : 'Play'} on:click|stopPropagation={togglePlay}>
							{#if $player.isPlaying}<Pause size={18} />{:else}<Play size={18} />{/if}
						</button>
						<button class="icon-button" aria-label="Next track" on:click|stopPropagation={playNext} disabled={$player.currentIndex >= $player.queue.length - 1}>
							<ChevronsRight size={18} />
						</button>
					</div>
					<div class="player-progress">
						<span>{formatDuration(Math.floor($player.currentTime))}</span>
						<input type="range" min="0" max="100" value={progress} on:click|stopPropagation on:input={seek} aria-label="Track progress" />
						<span>{formatDuration(Math.floor($player.duration || currentTrack.duration || 0))}</span>
					</div>
				</div>

				<div class="player-actions">
					<div class="volume-control">
						<Volume2 size={16} />
						<input type="range" min="0" max="1" step="0.01" value={$player.volume} on:click|stopPropagation on:input={(event) => setVolume(Number((event.target as HTMLInputElement).value))} aria-label="Volume" />
					</div>
					<button class="icon-button" aria-label="Lyrics" title="Lyrics coming later" disabled on:click|stopPropagation>
						<MessageSquareText size={18} />
					</button>
					<button class="icon-button" aria-label="Queue" on:click|stopPropagation={toggleQueue}>
						<ListMusic size={18} />
					</button>
				</div>
			</div>
		{:else}
			<div class="player-compact">
				<div class="player-art">
					<ImageWithFallback src={queueTrackImage(currentTrack)} alt={currentTrack.title} />
				</div>
				<div class="player-track">
					<strong>{currentTrack.title}</strong>
					<span>{currentTrack.artist}</span>
				</div>
				<button class="icon-button primary" aria-label={$player.isPlaying ? 'Pause' : 'Play'} on:click|stopPropagation={togglePlay}>
					{#if $player.isPlaying}<Pause size={18} />{:else}<Play size={18} />{/if}
				</button>
				<button class="icon-button" aria-label="Next track" on:click|stopPropagation={playNext} disabled={$player.currentIndex >= $player.queue.length - 1}>
					<ChevronsRight size={18} />
				</button>
			</div>
		{/if}

		{#if $player.queueOpen}
			<div class="queue-panel">
				<header>Queue</header>
				{#each $player.queue as track, index}
					<button class:active={index === $player.currentIndex} on:click|stopPropagation={() => playIndex(index)}>
						<div class="queue-art">
							<ImageWithFallback src={queueTrackImage(track)} alt={track.title} />
						</div>
						<span>
							<strong>{track.title}</strong>
							<small>{track.artist}</small>
						</span>
					</button>
				{/each}
			</div>
		{/if}
	</div>
{/if}
