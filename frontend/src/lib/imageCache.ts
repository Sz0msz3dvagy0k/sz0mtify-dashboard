import { browser } from '$app/environment';
import { getAuthToken } from '$lib/auth';
import { apiBase } from '$lib/format';
import { localImageObjectUrl } from '$lib/localMedia';

const CACHE_NAME = 'music-dashboard-images-v1';
const MAX_MEMORY_IMAGES = 600;
const MAX_DISK_IMAGES = 400;
const MAX_DISK_BYTES = 120 * 1024 * 1024;
const FAILURE_TTL_MS = 5 * 60 * 1000;
const DISK_PRUNE_INTERVAL_MS = 60 * 1000;

type ImageCacheEntry = {
	objectUrl?: string;
	promise?: Promise<string>;
	failedAt?: number;
	usedAt: number;
};

const memoryCache = new Map<string, ImageCacheEntry>();
let lastDiskPruneAt = 0;

export function cachedImageSrc(src: string | null): string | null {
	if (!src) return null;

	const cacheKey = dashboardApiImageUrl(src);
	if (!cacheKey) return src;

	const existing = memoryCache.get(cacheKey);
	if (!existing?.objectUrl) return null;

	existing.usedAt = Date.now();
	return existing.objectUrl;
}

export async function loadCachedImage(src: string): Promise<string> {
	const cacheKey = dashboardApiImageUrl(src);
	if (!cacheKey) return src;

	const localUrl = await localImageObjectUrl(cacheKey);
	if (localUrl) return localUrl;

	const existing = memoryCache.get(cacheKey);
	const now = Date.now();
	if (existing?.objectUrl) {
		existing.usedAt = now;
		return existing.objectUrl;
	}
	if (existing?.promise) {
		existing.usedAt = now;
		return existing.promise;
	}
	if (existing?.failedAt && now - existing.failedAt < FAILURE_TTL_MS) {
		throw new Error('cached_image_failed');
	}

	const entry: ImageCacheEntry = { usedAt: now };
	const promise = fetchCachedBlob(cacheKey)
		.then((blob) => {
			const objectUrl = URL.createObjectURL(blob);
			entry.objectUrl = objectUrl;
			entry.promise = undefined;
			entry.failedAt = undefined;
			entry.usedAt = Date.now();
			evictOldImages();
			return objectUrl;
		})
		.catch((error) => {
			entry.promise = undefined;
			entry.failedAt = Date.now();
			throw error;
		});

	entry.promise = promise;
	memoryCache.set(cacheKey, entry);
	return promise;
}

function dashboardApiImageUrl(src: string): string | null {
	if (!browser || !src) return null;

	let parsed: URL;
	try {
		parsed = new URL(src, window.location.href);
	} catch {
		return null;
	}

	if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return null;

	const base = new URL(apiBase() || window.location.origin, window.location.href);
	const basePath = base.pathname.replace(/\/$/, '');
	const apiPath = `${basePath}/api/`.replace(/\/{2,}/g, '/');

	if (parsed.origin !== base.origin) return null;
	if (!parsed.pathname.startsWith(apiPath)) return null;
	if (!/^\/.*\/(cover|artist-image)\//.test(parsed.pathname)) return null;

	return parsed.href;
}

async function fetchCachedBlob(url: string): Promise<Blob> {
	const headers = authHeaders();
	if ('caches' in window) {
		const cache = await caches.open(CACHE_NAME);
		const cached = await cache.match(url);
		if (cached?.ok) return cached.blob();

		const response = await fetch(url, { cache: 'force-cache', credentials: 'include', headers });
		if (!response.ok) throw new Error(`image_fetch_failed_${response.status}`);
		const blob = await response.blob();
		await cache.put(url, cacheableImageResponse(response, blob));
		void pruneDiskImageCache(cache);
		return blob;
	}

	const response = await fetch(url, { cache: 'force-cache', credentials: 'include', headers });
	if (!response.ok) throw new Error(`image_fetch_failed_${response.status}`);
	return response.blob();
}

function cacheableImageResponse(response: Response, blob: Blob): Response {
	const headers = new Headers(response.headers);
	headers.set('x-music-dashboard-cached-at', String(Date.now()));
	return new Response(blob, {
		headers,
		status: response.status,
		statusText: response.statusText
	});
}

function authHeaders(): HeadersInit | undefined {
	const token = getAuthToken();
	return token ? { authorization: `Bearer ${token}` } : undefined;
}

function evictOldImages() {
	if (memoryCache.size <= MAX_MEMORY_IMAGES) return;

	const entries = [...memoryCache.entries()]
		.filter(([, entry]) => entry.objectUrl && !entry.promise)
		.sort(([, a], [, b]) => a.usedAt - b.usedAt);

	for (const [key, entry] of entries.slice(0, memoryCache.size - MAX_MEMORY_IMAGES)) {
		if (entry.objectUrl) URL.revokeObjectURL(entry.objectUrl);
		memoryCache.delete(key);
	}
}

async function pruneDiskImageCache(cache: Cache) {
	const now = Date.now();
	if (now - lastDiskPruneAt < DISK_PRUNE_INTERVAL_MS) return;
	lastDiskPruneAt = now;

	const requests = await cache.keys();
	const entries = await Promise.all(
		requests.map(async (request) => {
			const response = await cache.match(request);
			if (!response) return null;
			const blob = await response.clone().blob();
			return {
				request,
				bytes: blob.size,
				storedAt: Number(response.headers.get('x-music-dashboard-cached-at') ?? response.headers.get('date') ?? 0)
			};
		})
	);

	const imageEntries = entries
		.filter((entry): entry is NonNullable<(typeof entries)[number]> => entry !== null)
		.sort((a, b) => a.storedAt - b.storedAt);

	let totalBytes = imageEntries.reduce((sum, entry) => sum + entry.bytes, 0);
	let totalImages = imageEntries.length;

	for (const entry of imageEntries) {
		if (totalImages <= MAX_DISK_IMAGES && totalBytes <= MAX_DISK_BYTES) break;
		await cache.delete(entry.request);
		totalBytes -= entry.bytes;
		totalImages -= 1;
	}
}
