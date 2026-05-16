import { browser } from '$app/environment';
import { Capacitor } from '@capacitor/core';
import { Directory, Filesystem } from '@capacitor/filesystem';
import { Preferences } from '@capacitor/preferences';
import { writable } from 'svelte/store';
import { getAuthToken, withStreamToken } from '$lib/auth';
import { apiBase, coverUrl } from '$lib/format';
import type {
	AlbumDetail,
	AlbumTuple,
	ArtistTuple,
	PlaylistDetail,
	PlaylistSummary,
	PlaylistTrack,
	SearchResult,
	StorageStats,
	TrackTuple
} from '$lib/types';
import type { QueueTrack } from '$lib/player';

export type LocalTrack = QueueTrack & {
	filePath: string;
	imagePath: string | null;
	sizeBytes: number;
	contentType: string | null;
	suffix: string | null;
	genre: string | null;
	trackNumber: number | null;
	discNumber: number | null;
	downloadedAt: string;
};

export type LocalAlbum = {
	id: number;
	title: string;
	artistName: string;
	year: number | null;
	genre: string | null;
	coverArtId: string | null;
	trackIds: number[];
	sourceTrackCount: number | null;
	downloadedAt: string;
};

export type LocalPlaylist = {
	id: string;
	name: string;
	durationSeconds: number;
	coverArtId: string | null;
	trackIds: number[];
	sourceSongCount: number | null;
	downloadedAt: string;
};

export type LocalImage = {
	key: string;
	path: string;
	contentType: string | null;
	sizeBytes: number;
	downloadedAt: string;
};

export type LocalMediaManifest = {
	version: 1;
	tracks: Record<string, LocalTrack>;
	albums: Record<string, LocalAlbum>;
	playlists: Record<string, LocalPlaylist>;
	images: Record<string, LocalImage>;
};

export type DownloadProgress = {
	total: number;
	completed: number;
	currentTitle: string;
};

type TrackDownloadContext = {
	album?: {
		id: number;
		title: string;
		artistName: string;
		year: number | null;
		genre: string | null;
		coverArtId: string | null;
		sourceTrackCount: number | null;
		trackNumber?: number | null;
		discNumber?: number | null;
	};
	playlist?: {
		id: string;
		name: string;
		durationSeconds: number;
		coverArtId: string | null;
		sourceSongCount: number | null;
		genre?: string | null;
	};
};

const manifestKey = 'archive.localMediaManifest.v1';
const browserObjectUrls = new Map<string, string>();

const emptyManifest = (): LocalMediaManifest => ({
	version: 1,
	tracks: {},
	albums: {},
	playlists: {},
	images: {}
});

let manifestCache: LocalMediaManifest | null = null;
let manifestPromise: Promise<LocalMediaManifest> | null = null;

export const localMedia = writable<LocalMediaManifest>(emptyManifest());

export async function loadLocalMedia(): Promise<LocalMediaManifest> {
	if (!browser) return emptyManifest();
	if (manifestCache) return manifestCache;
	if (manifestPromise) return manifestPromise;

	manifestPromise = readManifest().then((manifest) => {
		manifestCache = manifest;
		localMedia.set(manifest);
		return manifest;
	});
	return manifestPromise;
}

export async function hasLocalMedia(trackId: number): Promise<boolean> {
	const manifest = await loadLocalMedia();
	const track = manifest.tracks[String(trackId)];
	if (!track) return false;
	try {
		await Filesystem.stat({ path: track.filePath, directory: Directory.Data });
		return true;
	} catch {
		return false;
	}
}

export async function localTrackUrl(trackId: number): Promise<string | null> {
	const manifest = await loadLocalMedia();
	const track = manifest.tracks[String(trackId)];
	if (!track || !(await hasLocalMedia(trackId))) return null;

	if (Capacitor.isNativePlatform()) {
		const uri = await Filesystem.getUri({ path: track.filePath, directory: Directory.Data });
		return Capacitor.convertFileSrc(uri.uri);
	}

	return readFileObjectUrl(track.filePath, track.contentType ?? 'application/octet-stream');
}

export async function localImageObjectUrl(src: string | null | undefined): Promise<string | null> {
	if (!src) return null;
	const key = localImageKey(src);
	if (!key) return null;
	const manifest = await loadLocalMedia();
	const image = manifest.images[key];
	if (!image) return null;

	try {
		await Filesystem.stat({ path: image.path, directory: Directory.Data });
	} catch {
		return null;
	}

	if (Capacitor.isNativePlatform()) {
		const uri = await Filesystem.getUri({ path: image.path, directory: Directory.Data });
		return Capacitor.convertFileSrc(uri.uri);
	}

	return readFileObjectUrl(image.path, image.contentType ?? 'application/octet-stream');
}

export async function downloadTrack(track: QueueTrack, context: TrackDownloadContext = {}): Promise<LocalTrack> {
	const manifest = cloneManifest(await loadLocalMedia());
	const trackKey = String(track.id);
	const existing = manifest.tracks[trackKey];
	const localExists = existing ? await hasLocalMedia(track.id) : false;
	const image = await downloadCoverIfNeeded(manifest, track.coverArtId ?? context.album?.coverArtId ?? null);

	if (localExists) {
		const updated = {
			...existing,
			...track,
			imagePath: image?.path ?? existing.imagePath,
			genre: context.playlist?.genre ?? existing.genre ?? context.album?.genre ?? null,
			trackNumber: context.album?.trackNumber ?? existing.trackNumber ?? null,
			discNumber: context.album?.discNumber ?? existing.discNumber ?? null
		};
		manifest.tracks[trackKey] = updated;
		upsertContext(manifest, updated, context);
		await saveManifest(manifest);
		return updated;
	}

	const response = await fetch(await remoteLosslessStreamUrl(track.id), {
		credentials: 'include',
		headers: authHeaders()
	});
	if (!response.ok) throw new Error(`download_failed_${response.status}`);

	const blob = await response.blob();
	if (!blob.size) throw new Error('downloaded_media_empty');
	const contentType = cleanContentType(response.headers.get('content-type') ?? blob.type);
	const suffix = suffixFromHeaders(response.headers, contentType);
	const path = `local-media/tracks/${track.id}.${suffix}`;
	await writeBlob(path, blob);

	const localTrack: LocalTrack = {
		...track,
		filePath: path,
		imagePath: image?.path ?? null,
		sizeBytes: blob.size,
		contentType,
		suffix,
		genre: context.playlist?.genre ?? context.album?.genre ?? null,
		trackNumber: context.album?.trackNumber ?? null,
		discNumber: context.album?.discNumber ?? null,
		downloadedAt: new Date().toISOString()
	};

	manifest.tracks[trackKey] = localTrack;
	upsertContext(manifest, localTrack, context);
	await saveManifest(manifest);
	return localTrack;
}

export async function downloadAlbum(
	detail: AlbumDetail,
	artistName: string,
	progress?: (progress: DownloadProgress) => void
): Promise<void> {
	const album = detail.album;
	if (!album) throw new Error('album_not_found');
	const tracks = detail.tracks;
	for (let index = 0; index < tracks.length; index += 1) {
		const track = tracks[index];
		progress?.({ total: tracks.length, completed: index, currentTitle: track[1] });
		await downloadTrack(
			{
				id: track[0],
				title: track[1],
				artist: artistName,
				album: album[1],
				albumId: album[0],
				coverArtId: album[6],
				duration: track[4]
			},
			{
				album: {
					id: album[0],
					title: album[1],
					artistName,
					year: album[3],
					genre: album[4],
					coverArtId: album[6],
					sourceTrackCount: album[5],
					trackNumber: track[2],
					discNumber: track[3]
				}
			}
		);
		progress?.({ total: tracks.length, completed: index + 1, currentTitle: track[1] });
	}
}

export async function downloadPlaylist(
	detail: PlaylistDetail,
	progress?: (progress: DownloadProgress) => void
): Promise<void> {
	for (let index = 0; index < detail.tracks.length; index += 1) {
		const track = detail.tracks[index];
		progress?.({ total: detail.tracks.length, completed: index, currentTitle: track[1] });
		await downloadTrack(playlistTrackToQueueTrack(track, detail), {
			playlist: {
				id: detail.playlist.id,
				name: detail.playlist.name,
				durationSeconds: detail.playlist.duration_seconds,
				coverArtId: detail.playlist.cover_art_id,
				sourceSongCount: detail.playlist.song_count,
				genre: track[7]
			},
			album: track[3]
				? {
						id: track[3],
						title: track[4] ?? detail.playlist.name,
						artistName: track[2] ?? 'Unknown artist',
						year: null,
						genre: track[7],
						coverArtId: track[5] ?? detail.playlist.cover_art_id,
						sourceTrackCount: null
					}
				: undefined
		});
		progress?.({ total: detail.tracks.length, completed: index + 1, currentTitle: track[1] });
	}
}

export async function deleteLocalTrack(trackId: number): Promise<void> {
	const manifest = cloneManifest(await loadLocalMedia());
	const track = manifest.tracks[String(trackId)];
	if (!track) return;
	await deletePath(track.filePath);
	delete manifest.tracks[String(trackId)];

	for (const album of Object.values(manifest.albums)) {
		album.trackIds = album.trackIds.filter((id) => id !== trackId);
	}
	for (const playlist of Object.values(manifest.playlists)) {
		playlist.trackIds = playlist.trackIds.filter((id) => id !== trackId);
	}
	removeEmptyCollections(manifest);
	await cleanupUnreferencedImages(manifest);
	await saveManifest(manifest);
}

export async function deleteLocalAlbum(albumId: number): Promise<void> {
	const manifest = cloneManifest(await loadLocalMedia());
	const album = manifest.albums[String(albumId)];
	if (!album) return;
	delete manifest.albums[String(albumId)];
	for (const trackId of album.trackIds) {
		if (!trackReferencedElsewhere(manifest, trackId)) {
			await deletePath(manifest.tracks[String(trackId)]?.filePath);
			delete manifest.tracks[String(trackId)];
		}
	}
	await cleanupUnreferencedImages(manifest);
	await saveManifest(manifest);
}

export async function deleteLocalPlaylist(playlistId: string): Promise<void> {
	const manifest = cloneManifest(await loadLocalMedia());
	const playlist = manifest.playlists[playlistId];
	if (!playlist) return;
	delete manifest.playlists[playlistId];
	for (const trackId of playlist.trackIds) {
		if (!trackReferencedElsewhere(manifest, trackId)) {
			await deletePath(manifest.tracks[String(trackId)]?.filePath);
			delete manifest.tracks[String(trackId)];
		}
	}
	await cleanupUnreferencedImages(manifest);
	await saveManifest(manifest);
}

export async function offlineTracks(): Promise<TrackTuple[]> {
	const manifest = await loadLocalMedia();
	return Object.values(manifest.tracks)
		.sort((a, b) => a.title.localeCompare(b.title))
		.map((track) => [track.id, track.title, null, track.albumId, track.duration, track.genre]);
}

export async function offlineAlbums(): Promise<AlbumTuple[]> {
	const manifest = await loadLocalMedia();
	return Object.values(manifest.albums)
		.filter((album) => album.trackIds.some((id) => manifest.tracks[String(id)]))
		.sort((a, b) => a.title.localeCompare(b.title))
		.map((album) => [album.id, album.title, localArtistId(album.artistName), album.year, album.genre, album.coverArtId]);
}

export async function offlineAlbum(id: number): Promise<AlbumDetail> {
	const manifest = await loadLocalMedia();
	const album = manifest.albums[String(id)];
	if (!album) return { album: null, artist_name: null, tracks: [] };
	const tracks = album.trackIds
		.map((trackId) => manifest.tracks[String(trackId)])
		.filter(Boolean)
		.sort((a, b) => (a.discNumber ?? 0) - (b.discNumber ?? 0) || (a.trackNumber ?? 0) - (b.trackNumber ?? 0) || a.title.localeCompare(b.title))
		.map((track) => [track.id, track.title, track.trackNumber, track.discNumber, track.duration] as AlbumDetail['tracks'][number]);
	return {
		album: [album.id, album.title, localArtistId(album.artistName), album.year, album.genre, album.sourceTrackCount ?? tracks.length, album.coverArtId],
		artist_name: album.artistName,
		tracks
	};
}

export async function offlineArtists(): Promise<ArtistTuple[]> {
	const manifest = await loadLocalMedia();
	const artists = new Map<string, { albums: Set<number>; tracks: number; coverArtId: string | null }>();
	for (const track of Object.values(manifest.tracks)) {
		const entry = artists.get(track.artist) ?? { albums: new Set<number>(), tracks: 0, coverArtId: null };
		if (track.albumId) entry.albums.add(track.albumId);
		entry.tracks += 1;
		entry.coverArtId ??= track.coverArtId;
		artists.set(track.artist, entry);
	}
	return [...artists.entries()]
		.sort(([a], [b]) => a.localeCompare(b))
		.map(([name, entry]) => [localArtistId(name), name, entry.albums.size, entry.tracks, null, null, entry.coverArtId]);
}

export async function offlinePlaylists(): Promise<PlaylistSummary[]> {
	const manifest = await loadLocalMedia();
	return Object.values(manifest.playlists)
		.filter((playlist) => playlist.trackIds.some((id) => manifest.tracks[String(id)]))
		.sort((a, b) => a.name.localeCompare(b.name))
		.map((playlist) => playlistSummary(playlist, manifest));
}

export async function offlinePlaylist(id: string): Promise<PlaylistDetail> {
	const manifest = await loadLocalMedia();
	const playlist = manifest.playlists[id];
	if (!playlist) {
		return {
			playlist: { id, name: 'Downloaded playlist', song_count: 0, duration_seconds: 0, cover_art_id: null },
			tracks: []
		};
	}
	return {
		playlist: playlistSummary(playlist, manifest),
		tracks: playlist.trackIds.map((trackId) => localTrackToPlaylistTrack(manifest.tracks[String(trackId)])).filter(Boolean) as PlaylistTrack[]
	};
}

export async function offlineSearch(query: string): Promise<SearchResult> {
	const normalized = query.trim().toLowerCase();
	const manifest = await loadLocalMedia();
	const tracks = Object.values(manifest.tracks)
		.filter((track) => `${track.title} ${track.artist} ${track.album}`.toLowerCase().includes(normalized))
		.map((track) => [track.id, track.title, track.artist, track.albumId, track.album, track.coverArtId, track.duration] as SearchResult['tracks'][number]);
	const albums = Object.values(manifest.albums)
		.filter((album) => `${album.title} ${album.artistName}`.toLowerCase().includes(normalized))
		.map((album) => [album.id, album.title, album.artistName, album.coverArtId] as SearchResult['albums'][number]);
	const artists = (await offlineArtists())
		.filter((artist) => artist[1].toLowerCase().includes(normalized))
		.map((artist) => [artist[0], artist[1], artist[5], artist[6]] as SearchResult['artists'][number]);
	return { tracks, albums, artists };
}

export async function offlineStorageStats(): Promise<StorageStats> {
	const manifest = await loadLocalMedia();
	const tracks = Object.values(manifest.tracks);
	const total = tracks.reduce((sum, track) => sum + track.sizeBytes, 0);
	const duration = tracks.reduce((sum, track) => sum + (track.duration ?? 0), 0);
	const byFormat = groupTracks(tracks, (track) => track.suffix);
	const byContentType = groupTracks(tracks, (track) => track.contentType);
	const byGenre = groupTracks(tracks, (track) => track.genre);
	const byArtist = [...groupTracks(tracks, (track) => track.artist)].map(([name, bytes, count]) => [name ? localArtistId(name) : null, name, bytes, count] as [number | null, string | null, number, number]);
	const byAlbum = Object.values(manifest.albums).map((album) => {
		const albumTracks = album.trackIds.map((id) => manifest.tracks[String(id)]).filter(Boolean);
		return [
			album.id,
			album.title,
			localArtistId(album.artistName),
			albumTracks.reduce((sum, track) => sum + track.sizeBytes, 0),
			albumTracks.length
		] as [number | null, string | null, number | null, number, number];
	});
	const largestTracks = tracks
		.slice()
		.sort((a, b) => b.sizeBytes - a.sizeBytes)
		.map((track) => [track.id, track.title, null, track.albumId, track.sizeBytes, track.duration, track.suffix, track.contentType] as StorageStats['largest_tracks'][number]);
	const largestAlbums = byAlbum.slice().sort((a, b) => b[3] - a[3]);

	return {
		total_storage_bytes: total,
		tracks_size_bytes: total,
		average_track_size_bytes: tracks.length ? Math.round(total / tracks.length) : 0,
		average_mb_per_minute: duration ? total / 1_048_576 / (duration / 60) : null,
		size_by_format: byFormat,
		size_by_content_type: byContentType,
		size_by_artist: byArtist,
		size_by_album: byAlbum,
		size_by_genre: byGenre,
		largest_tracks: largestTracks,
		largest_albums: largestAlbums,
		extension_breakdown: byFormat,
		suspicious_large_tracks: [],
		generated_at: new Date().toISOString()
	};
}

export function localMediaTotals(manifest: LocalMediaManifest) {
	const tracks = Object.values(manifest.tracks);
	const bytes = tracks.reduce((sum, track) => sum + track.sizeBytes, 0);
	return {
		tracks: tracks.length,
		albums: Object.keys(manifest.albums).length,
		playlists: Object.keys(manifest.playlists).length,
		bytes
	};
}

async function remoteLosslessStreamUrl(trackId: number): Promise<string> {
	const response = await fetch(`${apiBase()}/api/auth/stream-token`, {
		method: 'POST',
		headers: { 'content-type': 'application/json', ...authHeaders() },
		credentials: 'include'
	});
	const payload = await response.json().catch(() => null);
	const token = payload?.ok ? payload.data?.token : null;
	if (!response.ok || !token) throw new Error('stream_token_unavailable');
	return withStreamToken(`${apiBase()}/api/tracks/${trackId}/stream?lossless=1`, token);
}

async function downloadCoverIfNeeded(manifest: LocalMediaManifest, coverArtId: string | null): Promise<LocalImage | null> {
	const src = coverUrl(coverArtId);
	const key = localImageKey(src);
	if (!src || !key) return null;
	const existing = manifest.images[key];
	if (existing) {
		try {
			await Filesystem.stat({ path: existing.path, directory: Directory.Data });
			return existing;
		} catch {
			delete manifest.images[key];
		}
	}

	const response = await fetch(src, { credentials: 'include', headers: authHeaders() });
	if (!response.ok) return null;
	const blob = await response.blob();
	if (!blob.size) return null;
	const contentType = cleanContentType(response.headers.get('content-type') ?? blob.type);
	const path = `local-media/images/${safePathSegment(key)}.${imageSuffix(contentType)}`;
	await writeBlob(path, blob);
	const image: LocalImage = {
		key,
		path,
		contentType,
		sizeBytes: blob.size,
		downloadedAt: new Date().toISOString()
	};
	manifest.images[key] = image;
	return image;
}

function upsertContext(manifest: LocalMediaManifest, track: LocalTrack, context: TrackDownloadContext) {
	if (context.album || track.albumId) {
		const albumId = context.album?.id ?? track.albumId;
		if (albumId) {
			const existing = manifest.albums[String(albumId)];
			const album: LocalAlbum = {
				id: albumId,
				title: context.album?.title ?? existing?.title ?? track.album,
				artistName: context.album?.artistName ?? existing?.artistName ?? track.artist,
				year: context.album?.year ?? existing?.year ?? null,
				genre: context.album?.genre ?? existing?.genre ?? track.genre,
				coverArtId: context.album?.coverArtId ?? existing?.coverArtId ?? track.coverArtId,
				trackIds: uniqueIds([...(existing?.trackIds ?? []), track.id]),
				sourceTrackCount: context.album?.sourceTrackCount ?? existing?.sourceTrackCount ?? null,
				downloadedAt: existing?.downloadedAt ?? new Date().toISOString()
			};
			manifest.albums[String(albumId)] = album;
		}
	}

	if (context.playlist) {
		const existing = manifest.playlists[context.playlist.id];
		manifest.playlists[context.playlist.id] = {
			id: context.playlist.id,
			name: context.playlist.name,
			durationSeconds: context.playlist.durationSeconds,
			coverArtId: context.playlist.coverArtId ?? existing?.coverArtId ?? track.coverArtId,
			trackIds: uniqueIds([...(existing?.trackIds ?? []), track.id]),
			sourceSongCount: context.playlist.sourceSongCount,
			downloadedAt: existing?.downloadedAt ?? new Date().toISOString()
		};
	}
}

function playlistTrackToQueueTrack(track: PlaylistTrack, detail: PlaylistDetail): QueueTrack {
	return {
		id: track[0],
		title: track[1],
		artist: track[2] ?? 'Unknown artist',
		album: track[4] ?? detail.playlist.name,
		albumId: track[3],
		coverArtId: track[5] ?? detail.playlist.cover_art_id,
		duration: track[6]
	};
}

function localTrackToPlaylistTrack(track: LocalTrack | undefined): PlaylistTrack | null {
	if (!track) return null;
	return [track.id, track.title, track.artist, track.albumId, track.album, track.coverArtId, track.duration, track.genre];
}

function playlistSummary(playlist: LocalPlaylist, manifest: LocalMediaManifest): PlaylistSummary {
	const tracks = playlist.trackIds.map((trackId) => manifest.tracks[String(trackId)]).filter(Boolean);
	return {
		id: playlist.id,
		name: playlist.name,
		song_count: tracks.length,
		duration_seconds: tracks.reduce((sum, track) => sum + (track.duration ?? 0), 0) || playlist.durationSeconds,
		cover_art_id: playlist.coverArtId ?? tracks.find((track) => track.coverArtId)?.coverArtId ?? null
	};
}

async function readManifest(): Promise<LocalMediaManifest> {
	try {
		const raw = Capacitor.isNativePlatform()
			? (await Preferences.get({ key: manifestKey })).value
			: localStorage.getItem(manifestKey);
		return normalizeManifest(raw ? JSON.parse(raw) : null);
	} catch (error) {
		console.warn('Unable to load local media manifest', error);
		return emptyManifest();
	}
}

async function saveManifest(manifest: LocalMediaManifest): Promise<void> {
	manifestCache = normalizeManifest(manifest);
	localMedia.set(manifestCache);
	const value = JSON.stringify(manifestCache);
	if (Capacitor.isNativePlatform()) {
		await Preferences.set({ key: manifestKey, value });
	} else {
		localStorage.setItem(manifestKey, value);
	}
}

function normalizeManifest(value: unknown): LocalMediaManifest {
	const candidate = value as Partial<LocalMediaManifest> | null;
	return {
		version: 1,
		tracks: candidate?.tracks && typeof candidate.tracks === 'object' ? candidate.tracks : {},
		albums: candidate?.albums && typeof candidate.albums === 'object' ? candidate.albums : {},
		playlists: candidate?.playlists && typeof candidate.playlists === 'object' ? candidate.playlists : {},
		images: candidate?.images && typeof candidate.images === 'object' ? candidate.images : {}
	};
}

function cloneManifest(manifest: LocalMediaManifest): LocalMediaManifest {
	return JSON.parse(JSON.stringify(manifest)) as LocalMediaManifest;
}

async function writeBlob(path: string, blob: Blob): Promise<void> {
	await Filesystem.writeFile({
		path,
		directory: Directory.Data,
		data: await blobToBase64(blob),
		recursive: true
	});
}

async function readFileObjectUrl(path: string, contentType: string): Promise<string> {
	const existing = browserObjectUrls.get(path);
	if (existing) return existing;
	const result = await Filesystem.readFile({ path, directory: Directory.Data });
	const data = typeof result.data === 'string' ? result.data : await blobToBase64(result.data);
	const blob = base64ToBlob(data, contentType);
	const objectUrl = URL.createObjectURL(blob);
	browserObjectUrls.set(path, objectUrl);
	return objectUrl;
}

function blobToBase64(blob: Blob): Promise<string> {
	return new Promise((resolve, reject) => {
		const reader = new FileReader();
		reader.onload = () => {
			const result = String(reader.result ?? '');
			resolve(result.includes(',') ? result.split(',')[1] : result);
		};
		reader.onerror = () => reject(reader.error ?? new Error('blob_read_failed'));
		reader.readAsDataURL(blob);
	});
}

function base64ToBlob(data: string, contentType: string): Blob {
	const clean = data.includes(',') ? data.split(',')[1] : data;
	const binary = atob(clean);
	const bytes = new Uint8Array(binary.length);
	for (let index = 0; index < binary.length; index += 1) {
		bytes[index] = binary.charCodeAt(index);
	}
	return new Blob([bytes], { type: contentType });
}

async function deletePath(path: string | null | undefined): Promise<void> {
	if (!path) return;
	try {
		await Filesystem.deleteFile({ path, directory: Directory.Data });
	} catch {
		return;
	}
	if (browserObjectUrls.has(path)) {
		URL.revokeObjectURL(browserObjectUrls.get(path) ?? '');
		browserObjectUrls.delete(path);
	}
}

async function cleanupUnreferencedImages(manifest: LocalMediaManifest): Promise<void> {
	const referenced = new Set(Object.values(manifest.tracks).map((track) => track.imagePath).filter(Boolean));
	for (const [key, image] of Object.entries(manifest.images)) {
		if (referenced.has(image.path)) continue;
		await deletePath(image.path);
		delete manifest.images[key];
	}
}

function removeEmptyCollections(manifest: LocalMediaManifest) {
	for (const [id, album] of Object.entries(manifest.albums)) {
		if (!album.trackIds.length) delete manifest.albums[id];
	}
	for (const [id, playlist] of Object.entries(manifest.playlists)) {
		if (!playlist.trackIds.length) delete manifest.playlists[id];
	}
}

function trackReferencedElsewhere(manifest: LocalMediaManifest, trackId: number): boolean {
	return (
		Object.values(manifest.albums).some((album) => album.trackIds.includes(trackId)) ||
		Object.values(manifest.playlists).some((playlist) => playlist.trackIds.includes(trackId))
	);
}

function groupTracks<T extends string | null>(
	tracks: LocalTrack[],
	key: (track: LocalTrack) => T
): [T, number, number][] {
	const grouped = new Map<T, { bytes: number; count: number }>();
	for (const track of tracks) {
		const name = key(track);
		const entry = grouped.get(name) ?? { bytes: 0, count: 0 };
		entry.bytes += track.sizeBytes;
		entry.count += 1;
		grouped.set(name, entry);
	}
	return [...grouped.entries()]
		.map(([name, entry]) => [name, entry.bytes, entry.count] as [T, number, number])
		.sort((a, b) => b[1] - a[1]);
}

function localImageKey(src: string | null): string | null {
	if (!browser || !src) return null;
	try {
		const parsed = new URL(src, window.location.href);
		if (!parsed.pathname.includes('/api/cover/')) return null;
		return decodeURIComponent(parsed.pathname.split('/api/cover/')[1] ?? '').trim() || null;
	} catch {
		return null;
	}
}

function safePathSegment(value: string): string {
	return value.replace(/[^a-zA-Z0-9._-]+/g, '_').slice(0, 160) || 'asset';
}

function cleanContentType(value: string | null): string | null {
	return value?.split(';')[0]?.trim() || null;
}

function suffixFromHeaders(headers: Headers, contentType: string | null): string {
	const disposition = headers.get('content-disposition') ?? '';
	const filename = disposition.match(/filename="([^"]+)"/)?.[1];
	const suffix = filename?.split('.').pop()?.toLowerCase();
	return safePathSegment(suffix || audioSuffix(contentType));
}

function audioSuffix(contentType: string | null): string {
	switch (contentType) {
		case 'audio/flac':
			return 'flac';
		case 'audio/mp4':
			return 'm4a';
		case 'audio/mpeg':
			return 'mp3';
		case 'audio/ogg':
			return 'ogg';
		case 'audio/opus':
			return 'opus';
		case 'audio/wav':
			return 'wav';
		default:
			return 'bin';
	}
}

function imageSuffix(contentType: string | null): string {
	switch (contentType) {
		case 'image/jpeg':
			return 'jpg';
		case 'image/png':
			return 'png';
		case 'image/webp':
			return 'webp';
		case 'image/gif':
			return 'gif';
		default:
			return 'img';
	}
}

function uniqueIds(values: number[]): number[] {
	return [...new Set(values.filter((value) => Number.isFinite(value)))];
}

function localArtistId(name: string): number {
	let hash = 0;
	for (let index = 0; index < name.length; index += 1) {
		hash = (hash * 31 + name.charCodeAt(index)) | 0;
	}
	return -Math.abs(hash || 1);
}

function authHeaders(): HeadersInit {
	const token = getAuthToken();
	return token ? { authorization: `Bearer ${token}` } : {};
}
