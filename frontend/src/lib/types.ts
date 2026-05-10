export type ApiEnvelope<T> = { ok: true; data: T } | { ok: false; error: string };

export type Overview = {
	total_tracks: number;
	total_albums: number;
	total_artists: number;
	total_plays: number;
};

export type TrackTuple = [number, string, number | null, number | null, number | null, string | null];
export type AlbumTuple = [number, string, number | null, number | null, string | null, string | null];
export type ArtistTuple = [
	number,
	string,
	number | null,
	number | null,
	number | null,
	string | null,
	string | null
];
export type GenreTuple = [number, string, number | null, number | null, number | null];

export type AlbumDetail = {
	album: [number, string, number | null, number | null, string | null, number | null, string | null] | null;
	tracks: [number, string, number | null, number | null][];
};

export type ArtistDetail = {
	artist: [number, string, number | null, number | null, number | null, string | null, string | null] | null;
	albums: [number, string, number | null, string | null][];
};

export type StorageStats = {
	total_storage_bytes: number;
	tracks_size_bytes: number;
	average_track_size_bytes: number;
	average_mb_per_minute: number | null;
	size_by_format: [string | null, number, number][];
	size_by_content_type: [string | null, number, number][];
	size_by_artist: [number | null, string | null, number, number][];
	size_by_album: [number | null, string | null, number | null, number, number][];
	size_by_genre: [string | null, number, number][];
	largest_tracks: [number, string, number | null, number | null, number, number | null, string | null, string | null][];
	largest_albums: [number | null, string | null, number | null, number, number][];
	extension_breakdown: [string | null, number, number][];
	suspicious_large_tracks: [number, string, number, number | null, string | null][];
	generated_at: string;
};

export type MetadataHealth = {
	total_tracks: number;
	missing_mbid: number;
	missing_genre: number;
};

export type ListeningStats = {
	data_source: 'plays_table' | 'subsonic_play_count' | 'mixed' | 'none';
	has_play_events: boolean;
	has_imported_play_counts: boolean;
	total_plays: number;
	top_tracks: [number, string, number][];
	top_artists: [number | null, string | null, number][];
	top_albums: [number | null, string | null, number][];
	recently_played: unknown[];
	timeline: [string, number][];
	generated_at: string;
};

export type DiscoveryItem = {
	id: number;
	local_artist_id: number | null;
	local_artist_name: string | null;
	discovered_artist_name: string | null;
	title: string | null;
	release_type: string | null;
	release_date: string | null;
	release_date_status: string;
	source: string | null;
	external_url: string | null;
	cover_url: string | null;
	already_in_library: boolean;
	match_status: string | null;
	confidence_score: number | null;
	reason: string | null;
	source_artist_name: string | null;
	source_artist_id: number | null;
};

export type DiscoveryList = {
	items: DiscoveryItem[];
	total: number;
	limit: number;
	offset: number;
	generated_at: string;
};

export type DiscoveryRefresh = {
	analyzed_artists: number;
	created_count: number;
	updated_count: number;
	skipped_count: number;
	error_count: number;
	errors: string[];
	generated_at: string;
};

export type SearchResult = {
	tracks: [number, string][];
	albums: [number, string][];
	artists: [number, string][];
};

export type SyncStatus = [number, string, string, string][];
