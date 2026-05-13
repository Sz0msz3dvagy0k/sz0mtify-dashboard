import { browser } from '$app/environment';
import { apiBase } from '$lib/format';

const CACHE_NAME = 'music-dashboard-images-v1';
const MAX_MEMORY_IMAGES = 240;
const FAILURE_TTL_MS = 5 * 60 * 1000;

type ImageCacheEntry = {
	objectUrl?: string;
	promise?: Promise<string>;
	failedAt?: number;
	usedAt: number;
};

const memoryCache = new Map<string, ImageCacheEntry>();

export async function loadCachedImage(src: string): Promise<string> {
	const cacheKey = dashboardApiImageUrl(src);
	if (!cacheKey) return src;

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
	if ('caches' in window) {
		const cache = await caches.open(CACHE_NAME);
		const cached = await cache.match(url);
		if (cached?.ok) return cached.blob();

		const response = await fetch(url, { cache: 'force-cache', credentials: 'include' });
		if (!response.ok) throw new Error(`image_fetch_failed_${response.status}`);
		await cache.put(url, response.clone());
		return response.blob();
	}

	const response = await fetch(url, { cache: 'force-cache', credentials: 'include' });
	if (!response.ok) throw new Error(`image_fetch_failed_${response.status}`);
	return response.blob();
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
