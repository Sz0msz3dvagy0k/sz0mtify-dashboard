import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { Capacitor } from '@capacitor/core';
import { withStreamToken } from '$lib/auth';
import { api } from '$lib/api';
import { apiBase, coverUrl } from '$lib/format';
import { localTrackNativeUri, localTrackUrl } from '$lib/localMedia';
import { currentNetworkType } from '$lib/mobileNetwork';

export type QueueTrack = {
	id: number;
	title: string;
	artist: string;
	album: string;
	albumId: number | null;
	coverArtId: string | null;
	duration: number | null;
};

type PlayerState = {
	queue: QueueTrack[];
	currentIndex: number;
	isPlaying: boolean;
	currentTime: number;
	duration: number;
	volume: number;
	queueOpen: boolean;
};

export type SongHistoryEntry = QueueTrack & {
	playedAt: string;
};

type StoredPlayerState = {
	queue: QueueTrack[];
	currentIndex: number;
	volume: number;
	wasPlaying: boolean;
};

const playerStorageKey = 'archive.playerState.v1';
const historyStorageKey = 'archive.songHistory.v1';
const maxHistoryEntries = 200;
const maxStoredQueueTracks = 100;

const initialState: PlayerState = {
	queue: [],
	currentIndex: -1,
	isPlaying: false,
	currentTime: 0,
	duration: 0,
	volume: 0.8,
	queueOpen: false
};

export const player = writable<PlayerState>(restorePlayerState());
export const songHistory = writable<SongHistoryEntry[]>([]);

let cachedStreamToken: { token: string; expires_at: number } | null = null;
let streamTokenRefreshTimer: number | null = null;
let historyRestored = false;

player.subscribe((state) => {
	if (!browser) return;
	const storedQueue = compactStoredQueue(state.queue, state.currentIndex);
	const stored: StoredPlayerState = {
		queue: storedQueue.queue,
		currentIndex: storedQueue.currentIndex,
		volume: state.volume,
		wasPlaying: state.isPlaying
	};
	writeJson(playerStorageKey, stored);
});

songHistory.subscribe((entries) => {
	if (!browser || !historyRestored) return;
	writeJson(historyStorageKey, entries.slice(0, maxHistoryEntries));
});

if (browser) {
	window.setTimeout(() => {
		historyRestored = true;
		songHistory.set(restoreSongHistory());
	}, 0);
}

export function playQueue(queue: QueueTrack[], startIndex = 0) {
	if (!queue.length) return;
	player.update((state) => ({
		...state,
		queue,
		currentIndex: Math.min(Math.max(startIndex, 0), queue.length - 1),
		isPlaying: true,
		currentTime: 0,
		duration: 0
	}));
}

export function queueTrackAtTop(track: QueueTrack) {
	player.update((state) => {
		const currentTrack = state.currentIndex >= 0 ? state.queue[state.currentIndex] : null;
		const queue = [track, ...state.queue.filter((queuedTrack) => queuedTrack.id !== track.id)];
		const currentIndex = currentTrack
			? queue.findIndex((queuedTrack) => queuedTrack.id === currentTrack.id)
			: state.queue.length
				? state.currentIndex
				: 0;
		return {
			...state,
			queue,
			currentIndex: currentIndex >= 0 ? currentIndex : state.currentIndex
		};
	});
}

export function playIndex(index: number) {
	player.update((state) => {
		if (!state.queue.length) return state;
		return {
			...state,
			currentIndex: Math.min(Math.max(index, 0), state.queue.length - 1),
			isPlaying: true,
			currentTime: 0,
			duration: 0
		};
	});
}

export function playNext() {
	player.update((state) => {
		if (!state.queue.length) return state;
		if (state.currentIndex >= state.queue.length - 1) {
			return { ...state, isPlaying: false };
		}
		const currentIndex = Math.min(state.currentIndex + 1, state.queue.length - 1);
		return { ...state, currentIndex, isPlaying: true, currentTime: 0, duration: 0 };
	});
}

export function playPrevious() {
	player.update((state) => {
		if (!state.queue.length) return state;
		const currentIndex = Math.max(state.currentIndex - 1, 0);
		return { ...state, currentIndex, isPlaying: true, currentTime: 0, duration: 0 };
	});
}

export function togglePlay() {
	player.update((state) => ({ ...state, isPlaying: !state.isPlaying }));
}

export function setPlaying(isPlaying: boolean) {
	player.update((state) => ({ ...state, isPlaying }));
}

export function setTime(currentTime: number, duration: number) {
	player.update((state) => ({ ...state, currentTime, duration: Number.isFinite(duration) ? duration : 0 }));
}

export function setVolume(volume: number) {
	player.update((state) => ({ ...state, volume: Math.min(Math.max(volume, 0), 1) }));
}

export function toggleQueue() {
	player.update((state) => ({ ...state, queueOpen: !state.queueOpen }));
}

export function closeQueue() {
	player.update((state) => (state.queueOpen ? { ...state, queueOpen: false } : state));
}

export async function warmStreamToken() {
	cachedStreamToken = await api.streamToken();
	scheduleStreamTokenRefresh();
}

async function streamToken() {
	const now = Math.floor(Date.now() / 1000);
	if (!cachedStreamToken || cachedStreamToken.expires_at - now < 30) {
		await warmStreamToken();
	}
	if (!cachedStreamToken) throw new Error('Unable to issue stream token');
	return cachedStreamToken.token;
}

function scheduleStreamTokenRefresh() {
	if (!browser || !cachedStreamToken) return;
	if (streamTokenRefreshTimer) clearTimeout(streamTokenRefreshTimer);
	const refreshInMs = Math.max(10_000, (cachedStreamToken.expires_at - Math.floor(Date.now() / 1000) - 60) * 1000);
	streamTokenRefreshTimer = window.setTimeout(() => {
		void warmStreamToken().catch((error) => console.warn('Unable to refresh stream token', error));
	}, refreshInMs);
}

export async function streamUrl(trackId: number, options: { lossless?: boolean } = {}) {
	const localUrl = await localTrackUrl(trackId);
	if (localUrl) return localUrl;

	const [token, networkType] = await Promise.all([streamToken(), currentNetworkType()]);
	const params = new URLSearchParams();
	if (networkType !== 'unknown') params.set('network', networkType);
	if (options.lossless) params.set('lossless', '1');
	const query = params.toString();
	return withStreamToken(`${apiBase()}/api/tracks/${trackId}/stream${query ? `?${query}` : ''}`, token);
}

export async function nativeLosslessAudioSource(trackId: number) {
	const localUri = await localTrackNativeUri(trackId);
	if (localUri) {
		return {
			assetPath: localUri,
			isUrl: true
		};
	}

	return {
		assetPath: await streamUrl(trackId, { lossless: true }),
		isUrl: true
	};
}

export function shouldUseNativeAudio() {
	return browser && Capacitor.isNativePlatform() && Capacitor.getPlatform() === 'ios';
}

export function queueTrackImage(track: QueueTrack | null | undefined) {
	return coverUrl(track?.coverArtId);
}

export function recordSongHistory(track: QueueTrack) {
	songHistory.update((entries) => {
		const playedAt = new Date().toISOString();
		const previous = entries[0];
		if (previous?.id === track.id && Date.now() - new Date(previous.playedAt).getTime() < 30_000) {
			if (!previous.coverArtId && track.coverArtId) {
				return [{ ...previous, coverArtId: track.coverArtId }, ...entries.slice(1)];
			}
			return entries;
		}
		const coverArtId = track.coverArtId ?? entries.find((entry) => entry.id === track.id)?.coverArtId ?? null;
		return [{ ...track, coverArtId, playedAt }, ...entries].slice(0, maxHistoryEntries);
	});
}

function restorePlayerState(): PlayerState {
	if (!browser) return initialState;
	const stored = readJson<StoredPlayerState>(playerStorageKey);
	if (!stored?.wasPlaying || !Array.isArray(stored.queue) || !stored.queue.length) return initialState;

	const storedQueue = stored.queue.filter(isQueueTrack).slice(0, maxStoredQueueTracks);
	if (!storedQueue.length) return initialState;
	const currentIndex = Math.min(Math.max(stored.currentIndex ?? 0, 0), storedQueue.length - 1);
	return {
		...initialState,
		queue: storedQueue,
		currentIndex,
		isPlaying: false,
		volume: clampVolume(stored.volume)
	};
}

function restoreSongHistory(): SongHistoryEntry[] {
	if (!browser) return [];
	const entries = readJson<SongHistoryEntry[]>(historyStorageKey);
	if (!Array.isArray(entries)) return [];
	return entries
		.filter((entry) => Number.isFinite(entry.id) && Boolean(entry.title) && Boolean(entry.playedAt))
		.slice(0, maxHistoryEntries);
}

function readJson<T>(key: string): T | null {
	try {
		const value = localStorage.getItem(key);
		return value ? (JSON.parse(value) as T) : null;
	} catch (error) {
		console.warn('Unable to read local player state', { key, error });
		return null;
	}
}

function writeJson(key: string, value: unknown) {
	try {
		localStorage.setItem(key, JSON.stringify(value));
	} catch (error) {
		console.warn('Unable to save local player state', { key, error });
	}
}

function compactStoredQueue(queue: QueueTrack[], currentIndex: number) {
	const compacted = compactQueue(queue);
	if (compacted.length <= maxStoredQueueTracks) {
		return {
			queue: compacted,
			currentIndex: Math.min(Math.max(currentIndex, 0), compacted.length - 1)
		};
	}
	const currentTrack = queue[currentIndex];
	const compactCurrentIndex = currentTrack ? compacted.findIndex((track) => track.id === currentTrack.id) : -1;
	const start = Math.max(0, Math.min(compactCurrentIndex - Math.floor(maxStoredQueueTracks / 2), compacted.length - maxStoredQueueTracks));
	const sliced = compacted.slice(start, start + maxStoredQueueTracks);
	return {
		queue: sliced,
		currentIndex: Math.max(0, sliced.findIndex((track) => currentTrack && track.id === currentTrack.id))
	};
}

function compactQueue(queue: QueueTrack[]): QueueTrack[] {
	return queue.filter(isQueueTrack).map((track) => ({
		id: track.id,
		title: track.title,
		artist: track.artist,
		album: track.album,
		albumId: typeof track.albumId === 'number' && Number.isFinite(track.albumId) ? track.albumId : null,
		coverArtId: track.coverArtId ?? null,
		duration: typeof track.duration === 'number' && Number.isFinite(track.duration) ? track.duration : null
	}));
}

function isQueueTrack(track: QueueTrack | unknown): track is QueueTrack {
	const candidate = track as Partial<QueueTrack> | null;
	return Boolean(candidate && Number.isFinite(candidate.id) && candidate.title && candidate.artist && candidate.album);
}

function clampVolume(value: number | null | undefined) {
	if (!Number.isFinite(value)) return initialState.volume;
	return Math.min(Math.max(value ?? initialState.volume, 0), 1);
}
