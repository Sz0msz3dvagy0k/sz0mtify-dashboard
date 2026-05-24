import { getApiBaseUrl } from '$lib/auth';

export function formatNumber(value: number | null | undefined): string {
	return new Intl.NumberFormat('en-US').format(value ?? 0);
}

export function formatBytes(value: number | null | undefined): string {
	const bytes = value ?? 0;
	if (bytes <= 0) return '0 B';
	const units = ['B', 'KB', 'MB', 'GB', 'TB'];
	const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
	return `${(bytes / 1024 ** index).toFixed(index < 2 ? 0 : 1)} ${units[index]}`;
}

export function formatDuration(seconds: number | null | undefined): string {
	const total = Math.max(0, seconds ?? 0);
	const hours = Math.floor(total / 3600);
	const minutes = Math.floor((total % 3600) / 60);
	const secs = total % 60;
	if (hours > 0) return `${hours}h ${minutes.toString().padStart(2, '0')}m`;
	return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

export function formatPercent(value: number | null | undefined, scale = 1): string {
	return `${Math.round((value ?? 0) * scale)}%`;
}

export function formatDate(value: string | null | undefined): string {
	if (!value) return 'date unknown';
	const date = new Date(value);
	if (Number.isNaN(date.getTime())) return value;
	return new Intl.DateTimeFormat('en-US', { month: 'short', day: 'numeric', year: 'numeric' }).format(date);
}

export function initials(value: string | null | undefined): string {
	return (value ?? '?')
		.split(/\s+/)
		.filter(Boolean)
		.slice(0, 2)
		.map((part) => part[0]?.toUpperCase())
		.join('');
}

export function coverUrl(coverArtId: string | null | undefined): string | null {
	if (!coverArtId) return null;
	return `${apiBase()}/api/cover/${encodeURIComponent(coverArtId)}`;
}

export function formatArtistBio(value: string | null | undefined): string {
	if (!value) return '';
	return value
		.split('Read more on Last.fm')[0]
		.split('User-contributed text is available')[0]
		.replace(/<[^>]*>/g, ' ')
		.replace(/&amp;/g, '&')
		.replace(/&quot;/g, '"')
		.replace(/&#39;|&apos;/g, "'")
		.replace(/&nbsp;/g, ' ')
		.replace(/\s+/g, ' ')
		.trim();
}

export function apiBase(): string {
	return getApiBaseUrl();
}
