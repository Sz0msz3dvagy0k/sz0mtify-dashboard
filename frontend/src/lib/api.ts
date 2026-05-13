import { apiBase } from './format';
import { clearAuthSession, getAuthToken } from './auth';
import type {
	AlbumDetail,
	AlbumTuple,
	ApiEnvelope,
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
	StorageStats,
	SyncStatus,
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

export const api = {
	health: () => fetch(`${apiBase()}/api/health`, { credentials: 'include' }).then((r) => r.json()),
	login: (body: { username: string; password: string }) =>
		request<AuthSession>('/api/auth/login', { method: 'POST', body: JSON.stringify(body) }),
	logout: () => request<{ status: string }>('/api/auth/logout', { method: 'POST' }),
	me: () => request<AuthUser>('/api/auth/me'),
	settings: () => request<[string, string][]>('/api/settings'),
	saveSettings: (body: Record<string, unknown>) =>
		request<Record<string, unknown>>('/api/settings', { method: 'POST', body: JSON.stringify(body) }),
	syncStatus: () => request<SyncStatus>('/api/sync/status'),
	syncAll: () => request<unknown>('/api/sync/all', { method: 'POST' }),
	syncSubsonic: () => request<unknown>('/api/sync/subsonic', { method: 'POST' }),
	syncLastfm: () => request<unknown>('/api/sync/lastfm', { method: 'POST' }),
	overview: () => request<Overview>('/api/library/overview'),
	tracks: () => request<TrackTuple[]>('/api/library/tracks'),
	albums: () => request<AlbumTuple[]>('/api/library/albums'),
	album: (id: number) => request<AlbumDetail>(`/api/library/albums/${id}`),
	nowPlaying: (id: number) => request<unknown>(`/api/tracks/${id}/now-playing`, { method: 'POST' }),
	artists: () => request<ArtistTuple[]>('/api/library/artists'),
	artist: (id: number) => request<ArtistDetail>(`/api/library/artists/${id}`),
	genres: () => request<GenreTuple[]>('/api/library/genres'),
	playlists: () => request<PlaylistSummary[]>('/api/playlists'),
	playlist: (id: string) => request<PlaylistDetail>(`/api/playlists/${encodeURIComponent(id)}`),
	audioQuality: () => request<[number | null, number | null, number][]>('/api/stats/audio-quality'),
	storage: () => request<StorageStats>('/api/stats/storage'),
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
	search: (q: string) => request<SearchResult>(`/api/search?q=${encodeURIComponent(q)}`)
};
