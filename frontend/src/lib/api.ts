import { apiBase } from './format';
import { clearAuthSession, getAuthToken } from './auth';
import {
	offlineAlbum,
	offlineAlbums,
	offlineArtists,
	offlinePlaylist,
	offlinePlaylists,
	offlineSearch,
	offlineStorageStats,
	offlineTracks
} from './localMedia';
import { isOfflineMode } from './mobileNetwork';
import type {
	AlbumDetail,
	AlbumTuple,
	ApiEnvelope,
	ActiveSession,
	AuthSession,
	AuthUser,
	ArtistDetail,
	ArtistTuple,
	DiscoveryList,
	DiscoveryRefresh,
	GenreTuple,
	ListeningStats,
	MetadataHealth,
	Overview,
	PlaylistDetail,
	PlaylistSummary,
	SearchResult,
	StreamToken,
	StorageStats,
	SyncStatus,
	TrackLyrics,
	TrackTuple
} from './types';

export class ApiError extends Error {
	constructor(
		message: string,
		public status = 0
	) {
		super(message);
	}
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
	const headers = new Headers(init?.headers);
	if (!headers.has('content-type')) headers.set('content-type', 'application/json');
	const token = getAuthToken();
	if (token) headers.set('authorization', `Bearer ${token}`);

	const response = await fetch(`${apiBase()}${path}`, {
		...init,
		headers,
		credentials: 'include',
	});
	const payload = (await response.json().catch(() => null)) as ApiEnvelope<T> | null;
	if (!response.ok) {
		if (response.status === 401) clearAuthSession();
		throw new ApiError(payload && 'error' in payload ? payload.error : response.statusText, response.status);
	}
	if (!payload) throw new ApiError('Empty response', response.status);
	if (!payload.ok) throw new ApiError(payload.error, response.status);
	return payload.data;
}

async function offlineFallback<T>(remote: () => Promise<T>, local: () => Promise<T>): Promise<T> {
	if (await isOfflineMode()) return local();
	try {
		return await remote();
	} catch (error) {
		if (isNetworkError(error) || (await isOfflineMode())) return local();
		throw error;
	}
}

function isNetworkError(error: unknown) {
	if (error instanceof TypeError) return true;
	if (!(error instanceof Error)) return false;
	return /failed to fetch|networkerror|load failed/i.test(error.message);
}

export const api = {
	health: () => fetch(`${apiBase()}/api/health`, { credentials: 'include' }).then((r) => r.json()),
	login: (body: { username: string; password: string }) =>
		request<AuthSession>('/api/auth/login', { method: 'POST', body: JSON.stringify(body) }),
	logout: () => request<{ status: string }>('/api/auth/logout', { method: 'POST' }),
	me: () => request<AuthUser>('/api/auth/me'),
	activeSessions: () => request<ActiveSession[]>('/api/auth/sessions'),
	streamToken: () => request<StreamToken>('/api/auth/stream-token', { method: 'POST' }),
	settings: () => request<[string, string][]>('/api/settings'),
	saveSettings: (body: Record<string, unknown>) =>
		request<Record<string, unknown>>('/api/settings', { method: 'POST', body: JSON.stringify(body) }),
	syncStatus: () => request<SyncStatus>('/api/sync/status'),
	syncAll: () => request<unknown>('/api/sync/all', { method: 'POST' }),
	syncSubsonic: () => request<unknown>('/api/sync/subsonic', { method: 'POST' }),
	syncLastfm: () => request<unknown>('/api/sync/lastfm', { method: 'POST' }),
	overview: () => request<Overview>('/api/library/overview'),
	tracks: () => offlineFallback(() => request<TrackTuple[]>('/api/library/tracks'), offlineTracks),
	albums: () => offlineFallback(() => request<AlbumTuple[]>('/api/library/albums'), offlineAlbums),
	album: (id: number) => offlineFallback(() => request<AlbumDetail>(`/api/library/albums/${id}`), () => offlineAlbum(id)),
	nowPlaying: (id: number) => request<unknown>(`/api/tracks/${id}/now-playing`, { method: 'POST' }),
	scrobble: (id: number) => request<unknown>(`/api/tracks/${id}/scrobble`, { method: 'POST' }),
	trackLyrics: (id: number) => request<TrackLyrics>(`/api/tracks/${id}/lyrics`),
	artists: () => offlineFallback(() => request<ArtistTuple[]>('/api/library/artists'), offlineArtists),
	artist: (id: number) => request<ArtistDetail>(`/api/library/artists/${id}`),
	genres: () => request<GenreTuple[]>('/api/library/genres'),
	playlists: () => offlineFallback(() => request<PlaylistSummary[]>('/api/playlists'), offlinePlaylists),
	playlist: (id: string) =>
		offlineFallback(() => request<PlaylistDetail>(`/api/playlists/${encodeURIComponent(id)}`), () => offlinePlaylist(id)),
	audioQuality: () => request<[number | null, number | null, number][]>('/api/stats/audio-quality'),
	storage: () => offlineFallback(() => request<StorageStats>('/api/stats/storage'), offlineStorageStats),
	metadataHealth: () => request<MetadataHealth>('/api/stats/metadata-health'),
	listening: () => request<ListeningStats>('/api/stats/listening'),
	timeline: () => request<[string, number][]>('/api/stats/timeline'),
	newReleases: (params = 'limit=50') => request<DiscoveryList>(`/api/discovery/new-releases?${params}`),
	missingAlbums: (params = 'limit=50') => request<DiscoveryList>(`/api/discovery/missing-albums?${params}`),
	similarArtists: (params = 'limit=50') => request<DiscoveryList>(`/api/discovery/similar-artists?${params}`),
	refreshDiscovery: (limit = 10) =>
		request<DiscoveryRefresh>(`/api/discovery/refresh?limit=${limit}`, { method: 'POST' }),
	rediscovery: () => request<{ tracks: [number, string, string | null, number | null, number, string | null][]; score_example: number }>('/api/recommendations/rediscovery'),
	currentRotation: () => request<[number, string, string | null, number | null, number | null][]>('/api/recommendations/current-rotation'),
	search: (q: string) => offlineFallback(() => request<SearchResult>(`/api/search?q=${encodeURIComponent(q)}`), () => offlineSearch(q))
};
