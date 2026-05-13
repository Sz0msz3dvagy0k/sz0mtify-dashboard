import { writable } from 'svelte/store';
import { withAccessToken } from '$lib/auth';
import { apiBase, coverUrl } from '$lib/format';

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

const initialState: PlayerState = {
	queue: [],
	currentIndex: -1,
	isPlaying: false,
	currentTime: 0,
	duration: 0,
	volume: 0.8,
	queueOpen: false
};

export const player = writable<PlayerState>(initialState);

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

export function streamUrl(trackId: number) {
	return withAccessToken(`${apiBase()}/api/tracks/${trackId}/stream`);
}

export function queueTrackImage(track: QueueTrack | null | undefined) {
	return coverUrl(track?.coverArtId);
}
