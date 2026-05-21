<script lang="ts">
	import { browser } from '$app/environment';
	import { onDestroy, onMount, tick } from 'svelte';
	import {
		ChevronsLeft,
		ChevronsRight,
		ListMusic,
		MessageSquareText,
		Pause,
		Play,
		Volume2
	} from 'lucide-svelte';
	import { ApiError, api } from '$lib/api';
	import ImageWithFallback from './ImageWithFallback.svelte';
	import { loadCachedImage } from '$lib/imageCache';
	import { formatDuration } from '$lib/format';
	import type { TrackLyrics } from '$lib/types';
	import {
		closeQueue,
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

	type PlayerDragMode = 'opening' | 'closing' | 'next' | 'previous';

	let playerElement: HTMLDivElement;
	let audio: HTMLAudioElement;
	let progressInput: HTMLInputElement;
	let lastTrackId: number | null = null;
	let registeredTrackId: number | null = null;
	let scrobbledTrackId: number | null = null;
	let scrobbleRequestTrackId: number | null = null;
	let lastScrobbleAttemptTrackId: number | null = null;
	let lastScrobbleAttemptAt = 0;
	let pendingAutoplayTrackId: number | null = null;
	let expanded = false;
	let playerTouchStartY = 0;
	let playerTouchStartX = 0;
	let playerTouchStartTime = 0;
	let playerDragOffset = 0;
	let playerSwipeOffset = 0;
	let playerDragMode: PlayerDragMode | null = null;
	let playerPointerId: number | null = null;
	let suppressNextPlayerClick = false;
	let mediaSessionTrackId: number | null = null;
	let playRequestTrackId: number | null = null;
	let streamRequestId = 0;
	let progressAnimationFrame: number | null = null;
	let displayedTrackId: number | null = null;
	let displayedCurrentTime = 0;
	let displayedDuration = 0;
	let lyricsOpen = false;
	let lyricsLoading = false;
	let lyrics: TrackLyrics | null = null;
	let lyricsError = '';
	let lyricsTrackId: number | null = null;
	let lyricsRequestId = 0;
	let lyricsLinesElement: HTMLDivElement | null = null;
	let lastScrolledLyricIndex = -1;
	$: currentTrack = $player.queue[$player.currentIndex] ?? null;
	$: previousTrack = $player.currentIndex > 0 ? $player.queue[$player.currentIndex - 1] : null;
	$: nextTrack = $player.currentIndex < $player.queue.length - 1 ? $player.queue[$player.currentIndex + 1] : null;
	$: progressDuration = displayedDuration || $player.duration || currentTrack?.duration || 0;
	$: progress = progressDuration > 0 ? (Math.min(displayedCurrentTime, progressDuration) / progressDuration) * 100 : 0;
	$: activeLyricIndex = lyricLineIndex(lyrics, displayedCurrentTime);
	$: verticalDragActive = playerDragMode === 'opening' || playerDragMode === 'closing';
	$: horizontalDragActive = playerDragMode === 'next' || playerDragMode === 'previous';
	$: visualExpanded = expanded || verticalDragActive;
	$: playerDragStyle = verticalDragActive
		? `transform: translate3d(0, ${Math.round(playerDragOffset)}px, 0);`
		: '';
	$: swipeTrackStyle = `transform: translate3d(${Math.round(playerSwipeOffset)}px, 0, 0);`;

	$: if (audio && currentTrack && currentTrack.id !== lastTrackId) {
		lastTrackId = currentTrack.id;
		scrobbledTrackId = null;
		lastScrobbleAttemptTrackId = null;
		lastScrobbleAttemptAt = 0;
		void loadTrackStream(currentTrack.id, ++streamRequestId);
	}

	$: if (audio && Math.abs(audio.volume - $player.volume) > 0.01) {
		audio.volume = $player.volume;
	}

	$: if (currentTrack?.id !== displayedTrackId) {
		displayedTrackId = currentTrack?.id ?? null;
		syncDisplayedTime();
	}

	$: if (currentTrack?.id !== lyricsTrackId) {
		lyricsTrackId = currentTrack?.id ?? null;
		lyrics = null;
		lyricsError = '';
		lyricsLoading = false;
		lastScrolledLyricIndex = -1;
		const requestId = ++lyricsRequestId;
		if (lyricsOpen && currentTrack) void loadLyrics(currentTrack.id, requestId);
	}

	$: if (lyricsOpen && lyrics?.synced && activeLyricIndex >= 0 && activeLyricIndex !== lastScrolledLyricIndex) {
		lastScrolledLyricIndex = activeLyricIndex;
		void scrollActiveLyricLine(activeLyricIndex);
	}

	$: if ($player.isPlaying) {
		startProgressAnimation();
	} else {
		stopProgressAnimation();
		syncDisplayedTime();
	}

	$: if (audio && currentTrack && $player.isPlaying && audio.paused && audio.readyState >= HTMLMediaElement.HAVE_METADATA) {
		void playAudio(currentTrack.id);
	}

	$: if (audio && !$player.isPlaying && !audio.paused) {
		audio.pause();
	}

	$: if (currentTrack && currentTrack.id !== mediaSessionTrackId) {
		mediaSessionTrackId = currentTrack.id;
		void updateMediaSessionMetadata(currentTrack);
	}

	$: if (!currentTrack && mediaSessionTrackId !== null) {
		mediaSessionTrackId = null;
		clearMediaSession();
	}

	$: updateMediaSessionPlaybackState();
	$: updateMediaSessionPositionState();

	function seek(event: Event) {
		const value = Number((event.target as HTMLInputElement).value);
		const duration = displayedDuration || $player.duration || currentTrack?.duration || 0;
		if (!audio || !duration) return;
		const nextTime = (value / 100) * duration;
		audio.currentTime = nextTime;
		displayedCurrentTime = nextTime;
		setTime(nextTime, duration);
		updateProgressControl();
	}

	function syncDisplayedTime() {
		const audioTime = audio && Number.isFinite(audio.currentTime) ? audio.currentTime : null;
		const audioDuration = audio && Number.isFinite(audio.duration) ? audio.duration : null;
		displayedCurrentTime = audioTime ?? $player.currentTime;
		displayedDuration = audioDuration ?? ($player.duration || currentTrack?.duration || 0);
		updateProgressControl();
	}

	function updateProgressControl() {
		if (!progressInput) return;
		const duration = displayedDuration || $player.duration || currentTrack?.duration || 0;
		const percent = duration > 0 ? (Math.min(displayedCurrentTime, duration) / duration) * 100 : 0;
		progressInput.value = String(percent);
		progressInput.style.setProperty('--progress', `${percent}%`);
	}

	function startProgressAnimation() {
		if (!browser || progressAnimationFrame !== null) return;
		const tick = () => {
			syncDisplayedTime();
			progressAnimationFrame = requestAnimationFrame(tick);
		};
		progressAnimationFrame = requestAnimationFrame(tick);
	}

	function stopProgressAnimation() {
		if (!browser || progressAnimationFrame === null) return;
		cancelAnimationFrame(progressAnimationFrame);
		progressAnimationFrame = null;
	}

	function handleGlobalKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && $player.queueOpen) {
			event.preventDefault();
			closeQueue();
			return;
		}
		if (event.code !== 'Space' || event.repeat || !currentTrack || isEditingTarget(event.target)) return;
		event.preventDefault();
		togglePlay();
	}

	function handleGlobalPointerDown(event: PointerEvent) {
		if (!$player.queueOpen) return;
		if (event.target instanceof Node && playerElement?.contains(event.target)) return;
		closeQueue();
	}

	async function updateMediaSessionMetadata(track: NonNullable<typeof currentTrack>) {
		if (!browser || !('mediaSession' in navigator) || !('MediaMetadata' in window)) return;

		navigator.mediaSession.metadata = new MediaMetadata({
			title: track.title,
			artist: track.artist,
			album: track.album,
			artwork: []
		});

		const artwork = await mediaSessionArtwork(track);
		if (currentTrack?.id !== track.id) return;

		navigator.mediaSession.metadata = new MediaMetadata({
			title: track.title,
			artist: track.artist,
			album: track.album,
			artwork
		});
	}

	function clearMediaSession() {
		if (!browser || !('mediaSession' in navigator)) return;
		navigator.mediaSession.metadata = null;
		navigator.mediaSession.playbackState = 'none';
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
			seekto: (details) => {
				if (!audio || typeof details.seekTime !== 'number') return;
				audio.currentTime = details.seekTime;
				const duration = displayedDuration || $player.duration || currentTrack?.duration || 0;
				setTime(details.seekTime, duration);
			}
		};

		for (const [action, handler] of Object.entries(handlers) as [MediaSessionAction, MediaSessionActionHandler][]) {
			setMediaSessionActionHandler(action, handler);
		}
		setMediaSessionActionHandler('seekbackward', null);
		setMediaSessionActionHandler('seekforward', null);
	}

	function setMediaSessionActionHandler(action: MediaSessionAction, handler: MediaSessionActionHandler | null) {
		try {
			navigator.mediaSession.setActionHandler(action, handler);
		} catch {
			// Ignore actions unsupported by the current browser/runtime.
		}
	}

	function isEditingTarget(target: EventTarget | null) {
		if (!(target instanceof HTMLElement)) return false;
		if (target.isContentEditable) return true;
		if (target === playerElement) return false;
		return Boolean(target.closest('input, textarea, select, button, a, [role="button"], [role="slider"]'));
	}

	function isPlayerControlTarget(target: EventTarget | null) {
		if (!(target instanceof HTMLElement)) return false;
		if (target.isContentEditable) return true;
		return Boolean(target.closest('input, textarea, select, button, a, [role="slider"]'));
	}

	async function loadTrackStream(trackId: number, requestId: number) {
		try {
			const src = await streamUrl(trackId);
			if (requestId !== streamRequestId || currentTrack?.id !== trackId) return;
			if ($player.isPlaying) pendingAutoplayTrackId = trackId;
			audio.src = src;
			audio.load();
		} catch (error) {
			console.warn('[player] stream load failed', { trackId, error });
			if (requestId === streamRequestId) setPlaying(false);
		}
	}

	async function playAudio(trackId: number) {
		if (playRequestTrackId === trackId || currentTrack?.id !== trackId) return;
		if (audio.readyState < HTMLMediaElement.HAVE_METADATA) {
			pendingAutoplayTrackId = trackId;
			return;
		}

		playRequestTrackId = trackId;
		try {
			await audio.play();
			pendingAutoplayTrackId = null;
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
		} finally {
			if (playRequestTrackId === trackId) playRequestTrackId = null;
		}
	}

	function maybeScrobbleCurrentTrack() {
		if (!audio || !currentTrack || !$player.isPlaying || audio.paused) return;
		const trackId = currentTrack.id;
		if (scrobbledTrackId === trackId || scrobbleRequestTrackId === trackId) return;
		if (lastScrobbleAttemptTrackId === trackId && Date.now() - lastScrobbleAttemptAt < 60_000) return;

		const duration = playbackDuration(currentTrack.duration);
		const currentTime = Number.isFinite(audio.currentTime) ? audio.currentTime : $player.currentTime;
		if (currentTime < scrobbleThreshold(duration)) return;

		scrobbleRequestTrackId = trackId;
		lastScrobbleAttemptTrackId = trackId;
		lastScrobbleAttemptAt = Date.now();
		void api
			.scrobble(trackId)
			.then(() => {
				if (currentTrack?.id === trackId) scrobbledTrackId = trackId;
			})
			.catch((error) => {
				console.warn('Unable to scrobble track', error);
			})
			.finally(() => {
				if (scrobbleRequestTrackId === trackId) scrobbleRequestTrackId = null;
			});
	}

	function playbackDuration(fallback: number | null | undefined) {
		if (audio && Number.isFinite(audio.duration) && audio.duration > 0) return audio.duration;
		if ($player.duration > 0) return $player.duration;
		return fallback && fallback > 0 ? fallback : 0;
	}

	function scrobbleThreshold(duration: number) {
		if (duration <= 0) return 30;
		return Math.max(30, Math.min(duration / 2, 240));
	}

	function handleMediaReady(eventName: string) {
		setTime(audio.currentTime, audio.duration);
		syncDisplayedTime();
		logAudioEvent(eventName);
		if (pendingAutoplayTrackId && currentTrack?.id === pendingAutoplayTrackId && $player.isPlaying) {
			void playAudio(pendingAutoplayTrackId);
		}
	}

	function handleAudioPlay(eventName: string) {
		logAudioEvent(eventName);
		if (currentTrack && !$player.isPlaying) setPlaying(true);
		updateMediaSessionPlaybackState();
		updateMediaSessionPositionState();
	}

	function handleAudioPause() {
		logAudioEvent('pause');
		if (
			currentTrack &&
			!audio.ended &&
			pendingAutoplayTrackId !== currentTrack.id &&
			playRequestTrackId !== currentTrack.id &&
			$player.isPlaying
		) {
			setPlaying(false);
		}
		updateMediaSessionPlaybackState();
		updateMediaSessionPositionState();
	}

	function handleAudioEnded() {
		const duration = playbackDuration(currentTrack?.duration);
		const currentTime = duration || (Number.isFinite(audio.currentTime) ? audio.currentTime : $player.currentTime);
		setTime(currentTime, duration);
		syncDisplayedTime();
		playNext();
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
		closeQueue();
		const target = event.currentTarget;
		if (target instanceof HTMLElement) target.setPointerCapture(event.pointerId);
		playerPointerId = event.pointerId;
		playerTouchStartX = event.clientX;
		playerTouchStartY = event.clientY;
		playerTouchStartTime = performance.now();
		playerDragOffset = expanded ? 0 : closedPlayerOffset();
		playerSwipeOffset = 0;
		playerDragMode = null;
	}

	function handlePlayerPointerMove(event: PointerEvent) {
		if (playerPointerId !== event.pointerId) return;
		const deltaX = event.clientX - playerTouchStartX;
		const deltaY = event.clientY - playerTouchStartY;
		const absX = Math.abs(deltaX);
		const absY = Math.abs(deltaY);

		if (!playerDragMode) {
			if (absX >= 10 && absX > absY * 1.15) {
				playerDragMode = deltaX < 0 ? 'next' : 'previous';
				suppressNextPlayerClick = true;
			} else if (absY >= 8 && absY > absX * 1.1 && !expanded && deltaY < 0) {
				playerDragMode = 'opening';
				suppressNextPlayerClick = true;
			} else if (absY >= 8 && absY > absX * 1.1 && expanded && deltaY > 0) {
				playerDragMode = 'closing';
				suppressNextPlayerClick = true;
			} else {
				return;
			}
		}

		event.preventDefault();
		if (isHorizontalDragMode(playerDragMode)) {
			playerSwipeOffset = swipeOffset(deltaX);
			return;
		}

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
		if (isHorizontalDragMode(playerDragMode)) {
			finishHorizontalSwipe(event);
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
		playerSwipeOffset = 0;
	}

	function finishHorizontalSwipe(event: PointerEvent) {
		const deltaX = event.clientX - playerTouchStartX;
		const elapsed = Math.max(1, performance.now() - playerTouchStartTime);
		const velocity = deltaX / elapsed;
		const releaseDistance = Math.min(150, swipeLimit() * 0.42);
		const shouldAdvance = deltaX < -releaseDistance || velocity < -0.35;
		const shouldGoBack = deltaX > releaseDistance || velocity > 0.35;

		if (shouldAdvance && canPlayNext()) {
			playerSwipeOffset = -swipeLimit();
			window.setTimeout(() => {
				playNext();
				resetPlayerDrag();
			}, 130);
			return;
		}

		if (shouldGoBack && canPlayPrevious()) {
			playerSwipeOffset = swipeLimit();
			window.setTimeout(() => {
				playPrevious();
				resetPlayerDrag();
			}, 130);
			return;
		}

		resetPlayerDrag();
	}

	function resetPlayerDrag() {
		playerDragMode = null;
		playerDragOffset = 0;
		playerSwipeOffset = 0;
		playerPointerId = null;
	}

	function closedPlayerOffset() {
		if (!browser) return 640;
		return Math.max(220, window.innerHeight - 92);
	}

	function clamp(value: number, min: number, max: number) {
		return Math.min(Math.max(value, min), max);
	}

	function swipeOffset(deltaX: number) {
		const limit = swipeLimit();
		const hasTarget = deltaX < 0 ? canPlayNext() : canPlayPrevious();
		if (hasTarget) return clamp(deltaX, -limit, limit);
		const resistance = Math.min(Math.abs(deltaX) * 0.32, limit * 0.28);
		return Math.sign(deltaX) * resistance;
	}

	function swipeLimit() {
		if (!browser) return 320;
		return playerElement?.getBoundingClientRect().width || Math.min(window.innerWidth * 0.9, 420);
	}

	function canPlayNext() {
		return $player.currentIndex >= 0 && $player.currentIndex < $player.queue.length - 1;
	}

	function canPlayPrevious() {
		return $player.currentIndex > 0;
	}

	function isHorizontalDragMode(mode: PlayerDragMode | null) {
		return mode === 'next' || mode === 'previous';
	}

	function handlePlayerFocusOut(event: FocusEvent) {
		if (!$player.queueOpen) return;
		if (event.relatedTarget instanceof Node && playerElement?.contains(event.relatedTarget)) return;
		closeQueue();
	}

	function handleQueueButtonClick(index: number) {
		playIndex(index);
		closeQueue();
	}

	function handleQueueToggle() {
		lyricsOpen = false;
		toggleQueue();
	}

	function toggleLyricsPanel() {
		lyricsOpen = !lyricsOpen;
		closeQueue();
		if (lyricsOpen && currentTrack && !lyrics && !lyricsLoading) {
			void loadLyrics(currentTrack.id, ++lyricsRequestId);
		}
	}

	async function loadLyrics(trackId: number, requestId: number) {
		lyricsLoading = true;
		lyricsError = '';
		try {
			const result = await api.trackLyrics(trackId);
			if (requestId !== lyricsRequestId || currentTrack?.id !== trackId) return;
			lyrics = result;
			lastScrolledLyricIndex = -1;
		} catch (error) {
			if (requestId !== lyricsRequestId || currentTrack?.id !== trackId) return;
			lyrics = null;
			lyricsError = error instanceof ApiError && error.status === 404
				? 'No lyrics found'
				: 'Unable to load lyrics';
			if (!(error instanceof ApiError && error.status === 404)) {
				console.warn('Unable to load lyrics', error);
			}
		} finally {
			if (requestId === lyricsRequestId && currentTrack?.id === trackId) {
				lyricsLoading = false;
			}
		}
	}

	function lyricLineIndex(trackLyrics: TrackLyrics | null, currentTime: number) {
		if (!trackLyrics?.synced) return -1;
		const currentMs = currentTime * 1000;
		let index = -1;
		trackLyrics.lines.forEach((line, lineIndex) => {
			if (line.start_ms !== null && line.start_ms <= currentMs) index = lineIndex;
		});
		return index;
	}

	function lyricsSourceLabel(source: string) {
		return source === 'lrclib' ? 'LRCLIB' : 'Subsonic';
	}

	async function scrollActiveLyricLine(index: number) {
		await tick();
		if (!lyricsLinesElement) return;
		const line = lyricsLinesElement.querySelector<HTMLElement>(`[data-lyric-index="${index}"]`);
		if (!line) return;

		const containerRect = lyricsLinesElement.getBoundingClientRect();
		const lineRect = line.getBoundingClientRect();
		const offset = lineRect.top - containerRect.top - lyricsLinesElement.clientHeight / 2 + lineRect.height / 2;
		lyricsLinesElement.scrollTo({
			top: lyricsLinesElement.scrollTop + offset,
			behavior: 'smooth'
		});
	}

	function logAudioEvent(eventName: string) {
		debugPlayer(`[player] audio ${eventName}`, {
			trackId: currentTrack?.id ?? null,
			currentSrc: redactStreamToken(audio.currentSrc),
			networkState: audio.networkState,
			readyState: audio.readyState,
			mediaError: describeMediaError(audio.error)
		});
	}

	function debugPlayer(message: string, data: unknown) {
		try {
			if (!browser || localStorage.getItem('archive.debugPlayer') !== '1') return;
			console.debug(message, data);
		} catch {
			return;
		}
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
		syncDisplayedTime();
		registerMediaSessionHandlers();
	});

	onDestroy(stopProgressAnimation);
</script>

<svelte:window on:keydown={handleGlobalKeydown} on:pointerdown|capture={handleGlobalPointerDown} />

<audio
	bind:this={audio}
	crossorigin="use-credentials"
	on:timeupdate={() => {
		setTime(audio.currentTime, audio.duration);
		syncDisplayedTime();
		maybeScrobbleCurrentTrack();
	}}
	on:loadedmetadata={() => handleMediaReady('loadedmetadata')}
	on:durationchange={() => handleMediaReady('durationchange')}
	on:canplay={() => handleMediaReady('canplay')}
	on:play={() => handleAudioPlay('play')}
	on:playing={() => handleAudioPlay('playing')}
	on:pause={handleAudioPause}
	on:stalled={() => logAudioEvent('stalled')}
	on:error={logAudioError}
	on:ended={handleAudioEnded}
></audio>

{#if currentTrack}
	<div
		bind:this={playerElement}
		class="player-bar"
		class:expanded={visualExpanded}
		class:dragging={playerPointerId !== null && playerDragMode !== null}
		class:queue-open={$player.queueOpen}
		class:lyrics-open={lyricsOpen}
		style={playerDragStyle}
		on:click={toggleExpanded}
		on:keydown={(event) => (event.key === 'Enter' || event.key === ' ') && toggleExpanded(event)}
		on:pointerdown={handlePlayerPointerDown}
		on:pointermove={handlePlayerPointerMove}
		on:pointerup={handlePlayerPointerUp}
		on:pointercancel={resetPlayerDrag}
		on:focusout={handlePlayerFocusOut}
		role="button"
		tabindex="0"
	>
		{#if visualExpanded}
			<button class="player-collapse" aria-label="Collapse player" on:click|stopPropagation={() => (expanded = false)}>×</button>
		{/if}
		<div class="player-swipe-viewport">
			<div class="player-swipe-track" class:dragging={playerPointerId !== null && horizontalDragActive} style={swipeTrackStyle}>
				<div class="player-swipe-panel">
					{#if previousTrack}
						<div class="player-adjacent" class:expanded={visualExpanded}>
							<div class={visualExpanded ? 'player-expanded-art' : 'player-art'}>
								<ImageWithFallback src={queueTrackImage(previousTrack)} alt={previousTrack.title} />
							</div>
							<div class={visualExpanded ? 'player-expanded-title' : 'player-track'}>
								<strong>{previousTrack.title}</strong>
								<span>{previousTrack.artist} · {previousTrack.album}</span>
							</div>
						</div>
					{/if}
				</div>
				<div class="player-swipe-panel">
					{#if visualExpanded}
			<div class="player-expanded-art">
				<ImageWithFallback src={queueTrackImage(currentTrack)} alt={currentTrack.title} />
			</div>
			<div class="player-expanded-main">
				<div class="player-expanded-title">
					<strong>{currentTrack.title}</strong>
					<span>{currentTrack.artist} · {currentTrack.album}</span>
				</div>

				{#if lyricsOpen}
					<div class="player-lyrics-panel" role="region" aria-label="Lyrics" on:pointerdown|stopPropagation>
						<header>
							<strong>Lyrics</strong>
							{#if lyrics}
								<small>{lyricsSourceLabel(lyrics.source)}{lyrics.synced ? ' - synced' : ''}</small>
							{/if}
						</header>
						{#if lyricsLoading}
							<div class="lyrics-status">Loading lyrics...</div>
						{:else if lyricsError}
							<div class="lyrics-status">{lyricsError}</div>
						{:else if lyrics?.instrumental}
							<div class="lyrics-status">Instrumental</div>
						{:else if lyrics && lyrics.lines.length}
							<div bind:this={lyricsLinesElement} class="lyrics-lines" class:synced={lyrics.synced}>
								{#each lyrics.lines as line, index}
									<p data-lyric-index={index} class:active={index === activeLyricIndex}>{line.text}</p>
								{/each}
							</div>
						{/if}
					</div>
				{/if}

				<div class="player-expanded-controls">
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
					</div>

					<div class="player-actions">
						<div class="volume-control">
							<Volume2 size={16} />
							<input type="range" min="0" max="1" step="0.01" value={$player.volume} on:click|stopPropagation on:input={(event) => setVolume(Number((event.target as HTMLInputElement).value))} aria-label="Volume" />
						</div>
						<button class="icon-button" class:active={lyricsOpen} aria-label="Lyrics" aria-pressed={lyricsOpen} on:click|stopPropagation={toggleLyricsPanel}>
							<MessageSquareText size={18} />
						</button>
						<button class="icon-button" aria-label="Queue" on:click|stopPropagation={handleQueueToggle}>
							<ListMusic size={18} />
						</button>
					</div>
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
					<div class="player-compact-buttons">
						<button class="icon-button desktop-player-control" aria-label="Previous track" on:click|stopPropagation={playPrevious} disabled={$player.currentIndex <= 0}>
							<ChevronsLeft size={18} />
						</button>
						<button class="icon-button primary" aria-label={$player.isPlaying ? 'Pause' : 'Play'} on:click|stopPropagation={togglePlay}>
							{#if $player.isPlaying}<Pause size={18} />{:else}<Play size={18} />{/if}
						</button>
						<button class="icon-button" aria-label="Next track" on:click|stopPropagation={playNext} disabled={$player.currentIndex >= $player.queue.length - 1}>
							<ChevronsRight size={18} />
						</button>
					</div>
					<div class="volume-control player-compact-volume desktop-player-control">
						<Volume2 size={16} />
						<input type="range" min="0" max="1" step="0.01" value={$player.volume} on:click|stopPropagation on:input={(event) => setVolume(Number((event.target as HTMLInputElement).value))} aria-label="Volume" />
					</div>
					<button class="icon-button desktop-player-control" aria-label="Queue" on:click|stopPropagation={handleQueueToggle}>
						<ListMusic size={18} />
					</button>
				</div>
			{/if}
				</div>
				<div class="player-swipe-panel">
					{#if nextTrack}
						<div class="player-adjacent" class:expanded={visualExpanded}>
							<div class={visualExpanded ? 'player-expanded-art' : 'player-art'}>
								<ImageWithFallback src={queueTrackImage(nextTrack)} alt={nextTrack.title} />
							</div>
							<div class={visualExpanded ? 'player-expanded-title' : 'player-track'}>
								<strong>{nextTrack.title}</strong>
								<span>{nextTrack.artist} · {nextTrack.album}</span>
							</div>
						</div>
					{/if}
				</div>
			</div>
		</div>

		<div class="player-progress">
			<span>{formatDuration(Math.floor($player.currentTime))}</span>
			<input bind:this={progressInput} type="range" min="0" max="100" step="any" value={progress} style={`--progress: ${progress}%;`} on:click|stopPropagation on:input={seek} aria-label="Track progress" />
			<span>{formatDuration(Math.floor($player.duration || currentTrack.duration || 0))}</span>
		</div>

		{#if $player.queueOpen}
			<div class="queue-panel" role="dialog" aria-label="Queue" tabindex="-1" on:pointerdown|stopPropagation>
				<header>Queue</header>
				{#each $player.queue as track, index}
					<button class:active={index === $player.currentIndex} on:click|stopPropagation={() => handleQueueButtonClick(index)}>
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
