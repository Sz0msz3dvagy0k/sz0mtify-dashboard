import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { withStreamToken } from '$lib/auth';
import { api } from '$lib/api';
import { apiBase, coverUrl } from '$lib/format';
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
export const songHistory = writable<SongHistoryEntry[]>(restoreSongHistory());

let cachedStreamToken: { token: string; expires_at: number } | null = null;

player.subscribe((state) => {
	if (!browser) return;
	const stored: StoredPlayerState = {
		queue: state.queue,
		currentIndex: state.currentIndex,
		volume: state.volume,
		wasPlaying: state.isPlaying
	};
	writeJson(playerStorageKey, stored);
});

songHistory.subscribe((entries) => {
	if (!browser) return;
	writeJson(historyStorageKey, entries.slice(0, maxHistoryEntries));
});

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
}

async function streamToken() {
	const now = Math.floor(Date.now() / 1000);
	if (!cachedStreamToken || cachedStreamToken.expires_at - now < 30) {
		await warmStreamToken();
	}
	if (!cachedStreamToken) throw new Error('Unable to issue stream token');
	return cachedStreamToken.token;
}

export async function streamUrl(trackId: number) {
	const [token, networkType] = await Promise.all([streamToken(), currentNetworkType()]);
	const params = new URLSearchParams();
	if (networkType !== 'unknown') params.set('network', networkType);
	const query = params.toString();
	return withStreamToken(`${apiBase()}/api/tracks/${trackId}/stream${query ? `?${query}` : ''}`, token);
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

	const currentIndex = Math.min(Math.max(stored.currentIndex ?? 0, 0), stored.queue.length - 1);
	return {
		...initialState,
		queue: stored.queue,
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

function clampVolume(value: number | null | undefined) {
	if (!Number.isFinite(value)) return initialState.volume;
	return Math.min(Math.max(value ?? initialState.volume, 0), 1);
}
