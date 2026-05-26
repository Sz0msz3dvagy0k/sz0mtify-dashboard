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
	TrackDetail,
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

const READ_CACHE_TTL_MS = 30 * 1000;
const readCache = new Map<string, { expiresAt: number; promise: Promise<unknown> }>();

type RequestOptions = {
	cacheTtlMs?: number;
};

async function request<T>(path: string, init?: RequestInit, options: RequestOptions = {}): Promise<T> {
	const method = init?.method?.toUpperCase() ?? 'GET';
	const cacheTtlMs = method === 'GET' ? options.cacheTtlMs : undefined;
	if (cacheTtlMs) {
		const cacheKey = `${method}:${path}`;
		const cached = readCache.get(cacheKey);
		if (cached && cached.expiresAt > Date.now()) return cached.promise as Promise<T>;

		const promise = performRequest<T>(path, init).catch((error) => {
			readCache.delete(cacheKey);
			throw error;
		});
		readCache.set(cacheKey, { expiresAt: Date.now() + cacheTtlMs, promise });
		return promise;
	}

	if (method !== 'GET') readCache.clear();
	return performRequest<T>(path, init);
}

async function performRequest<T>(path: string, init?: RequestInit): Promise<T> {
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
		if (response.status === 401) {
			readCache.clear();
			clearAuthSession();
		}
		throw new ApiError(payload && 'error' in payload ? payload.error : response.statusText, response.status);
	}
	if (!payload) throw new ApiError('Empty response', response.status);
	if (!payload.ok) throw new ApiError(payload.error, response.status);
	return payload.data;
}

function cachedRequest<T>(path: string, cacheTtlMs = READ_CACHE_TTL_MS): Promise<T> {
	return request<T>(path, undefined, { cacheTtlMs });
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
	deleteSession: (sessionId: string) =>
		request<{ status: string }>(`/api/auth/sessions/${encodeURIComponent(sessionId)}`, { method: 'DELETE' }),
	streamToken: () => request<StreamToken>('/api/auth/stream-token', { method: 'POST' }),
	settings: () => request<[string, string][]>('/api/settings'),
	saveSettings: (body: Record<string, unknown>) =>
		request<Record<string, unknown>>('/api/settings', { method: 'POST', body: JSON.stringify(body) }),
	syncStatus: () => request<SyncStatus>('/api/sync/status'),
	checkScan: () => request<unknown>('/api/sync/check-scan', { method: 'POST' }),
	syncAll: () => request<unknown>('/api/sync/all', { method: 'POST' }),
	syncSubsonic: () => request<unknown>('/api/sync/subsonic', { method: 'POST' }),
	syncLastfm: () => request<unknown>('/api/sync/lastfm', { method: 'POST' }),
	overview: () => cachedRequest<Overview>('/api/library/overview'),
	tracks: () => offlineFallback(() => cachedRequest<TrackTuple[]>('/api/library/tracks'), offlineTracks),
	track: (id: number) => cachedRequest<TrackDetail>(`/api/library/tracks/${id}`),
	albums: () => offlineFallback(() => cachedRequest<AlbumTuple[]>('/api/library/albums'), offlineAlbums),
	album: (id: number) => offlineFallback(() => cachedRequest<AlbumDetail>(`/api/library/albums/${id}`), () => offlineAlbum(id)),
	nowPlaying: (id: number) => request<unknown>(`/api/tracks/${id}/now-playing`, { method: 'POST' }),
	scrobble: (id: number) => request<unknown>(`/api/tracks/${id}/scrobble`, { method: 'POST' }),
	trackLyrics: (id: number) => request<TrackLyrics>(`/api/tracks/${id}/lyrics`),
	artists: () => offlineFallback(() => cachedRequest<ArtistTuple[]>('/api/library/artists'), offlineArtists),
	artist: (id: number) => cachedRequest<ArtistDetail>(`/api/library/artists/${id}`),
	genres: () => cachedRequest<GenreTuple[]>('/api/library/genres'),
	playlists: () => offlineFallback(() => cachedRequest<PlaylistSummary[]>('/api/playlists'), offlinePlaylists),
	playlist: (id: string) =>
		offlineFallback(() => cachedRequest<PlaylistDetail>(`/api/playlists/${encodeURIComponent(id)}`), () => offlinePlaylist(id)),
	addTrackToPlaylist: (playlistId: string, trackId: number) =>
		request<{ status: string }>(`/api/playlists/${encodeURIComponent(playlistId)}/tracks`, {
			method: 'POST',
			body: JSON.stringify({ track_id: trackId })
		}),
	audioQuality: () => cachedRequest<[number | null, number | null, number][]>('/api/stats/audio-quality'),
	storage: () => offlineFallback(() => cachedRequest<StorageStats>('/api/stats/storage'), offlineStorageStats),
	metadataHealth: () => cachedRequest<MetadataHealth>('/api/stats/metadata-health'),
	listening: () => cachedRequest<ListeningStats>('/api/stats/listening'),
	timeline: () => cachedRequest<[string, number][]>('/api/stats/timeline'),
	newReleases: (params = 'limit=50') => cachedRequest<DiscoveryList>(`/api/discovery/new-releases?${params}`),
	missingAlbums: (params = 'limit=50') => cachedRequest<DiscoveryList>(`/api/discovery/missing-albums?${params}`),
	similarArtists: (params = 'limit=50') => cachedRequest<DiscoveryList>(`/api/discovery/similar-artists?${params}`),
	refreshDiscovery: (limit = 10) =>
		request<DiscoveryRefresh>(`/api/discovery/refresh?limit=${limit}`, { method: 'POST' }),
	rediscovery: () => cachedRequest<{ tracks: [number, string, string | null, number | null, number, string | null][]; score_example: number }>('/api/recommendations/rediscovery'),
	currentRotation: () => cachedRequest<[number, string, string | null, number | null, number | null][]>('/api/recommendations/current-rotation'),
	search: (q: string) => offlineFallback(() => request<SearchResult>(`/api/search?q=${encodeURIComponent(q)}`), () => offlineSearch(q))
};
