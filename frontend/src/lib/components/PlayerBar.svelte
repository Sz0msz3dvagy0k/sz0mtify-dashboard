<script lang="ts">
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
	import { formatDuration } from '$lib/format';
	import {
		playIndex,
		playNext,
		playPrevious,
		player,
		queueTrackImage,
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
	let streamRequestId = 0;
	$: currentTrack = $player.queue[$player.currentIndex] ?? null;
	$: progress = $player.duration > 0 ? ($player.currentTime / $player.duration) * 100 : 0;

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

	function isEditingTarget(target: EventTarget | null) {
		if (!(target instanceof HTMLElement)) return false;
		if (target.isContentEditable) return true;
		return Boolean(target.closest('input, textarea, select, button, a, [role="button"], [role="slider"]'));
	}

	async function loadTrackStream(trackId: number, requestId: number) {
		try {
			const src = await streamUrl(trackId);
			if (requestId !== streamRequestId || currentTrack?.id !== trackId) return;
			audio.src = src;
			audio.load();
			if ($player.isPlaying) void playAudio(trackId);
		} catch {
			if (requestId === streamRequestId) setPlaying(false);
		}
	}

	async function playAudio(trackId: number) {
		try {
			await audio.play();
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
		} catch {
			setPlaying(false);
		}
	}

	onMount(() => {
		if (audio) audio.volume = $player.volume;
	});
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

<audio
	bind:this={audio}
	crossorigin="use-credentials"
	on:timeupdate={() => setTime(audio.currentTime, audio.duration)}
	on:loadedmetadata={() => setTime(audio.currentTime, audio.duration)}
	on:ended={playNext}
></audio>

{#if currentTrack}
	<div class="player-bar">
		<div class="player-track">
			<div class="player-art">
				<ImageWithFallback src={queueTrackImage(currentTrack)} alt={currentTrack.title} />
			</div>
			<div>
				<strong>{currentTrack.title}</strong>
				<span>{currentTrack.artist} · {currentTrack.album}</span>
			</div>
		</div>

		<div class="player-center">
			<div class="player-buttons">
				<button class="icon-button" aria-label="Previous track" on:click={playPrevious} disabled={$player.currentIndex <= 0}>
					<ChevronsLeft size={18} />
				</button>
				<button class="icon-button primary" aria-label={$player.isPlaying ? 'Pause' : 'Play'} on:click={togglePlay}>
					{#if $player.isPlaying}<Pause size={18} />{:else}<Play size={18} />{/if}
				</button>
				<button class="icon-button" aria-label="Next track" on:click={playNext} disabled={$player.currentIndex >= $player.queue.length - 1}>
					<ChevronsRight size={18} />
				</button>
			</div>
			<div class="player-progress">
				<span>{formatDuration(Math.floor($player.currentTime))}</span>
				<input type="range" min="0" max="100" value={progress} on:input={seek} aria-label="Track progress" />
				<span>{formatDuration(Math.floor($player.duration || currentTrack.duration || 0))}</span>
			</div>
		</div>

		<div class="player-actions">
			<div class="volume-control">
				<Volume2 size={16} />
				<input type="range" min="0" max="1" step="0.01" value={$player.volume} on:input={(event) => setVolume(Number((event.target as HTMLInputElement).value))} aria-label="Volume" />
			</div>
			<button class="icon-button" aria-label="Lyrics" title="Lyrics coming later" disabled>
				<MessageSquareText size={18} />
			</button>
			<button class="icon-button" aria-label="Queue" on:click={toggleQueue}>
				<ListMusic size={18} />
			</button>
		</div>

		{#if $player.queueOpen}
			<div class="queue-panel">
				<header>Queue</header>
				{#each $player.queue as track, index}
					<button class:active={index === $player.currentIndex} on:click={() => playIndex(index)}>
						<span>{track.title}</span>
						<small>{track.artist}</small>
					</button>
				{/each}
			</div>
		{/if}
	</div>
{/if}
