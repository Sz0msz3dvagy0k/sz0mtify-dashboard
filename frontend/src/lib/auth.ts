import { browser } from '$app/environment';
import { Capacitor } from '@capacitor/core';
import { Preferences } from '@capacitor/preferences';
import { writable } from 'svelte/store';
import type { AuthSession } from '$lib/types';

const STORAGE_KEY = 'music-dashboard.auth';
const NATIVE_STORAGE_KEY = 'music-dashboard.native-auth';
const API_BASE_STORAGE_KEY = 'music-dashboard.api-base';
const apiBasePattern = /^https?:\/\/(?:localhost|127(?:\.\d{1,3}){3}|\[(?:[a-f0-9:]+)\]|[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9]))*)(?::\d{1,5})?$/i;

let currentSession: AuthSession | null = null;
let currentApiBaseUrl = '';
export const authSession = writable<AuthSession | null>(null);

authSession.subscribe((value) => {
	currentSession = value;
});

export async function loadStoredSession(): Promise<AuthSession | null> {
	if (!browser) return null;

	try {
		const raw = await readStoredSession();
		if (!raw) return null;
		const session = JSON.parse(raw) as AuthSession & { apiBaseUrl?: string };
		if (!session.username || session.expires_at <= Math.floor(Date.now() / 1000)) {
			clearAuthSession();
			return null;
		}
		setApiBaseUrl(session.apiBaseUrl ?? readStoredApiBase() ?? '');
		authSession.set(session);
		return session;
	} catch {
		clearAuthSession();
		return null;
	}
}

export async function saveAuthSession(session: AuthSession, apiBaseUrl = currentApiBaseUrl) {
	const normalizedApiBaseUrl = normalizeApiBaseUrl(apiBaseUrl);
	const storedSession = {
		username: session.username,
		expires_at: session.expires_at,
		apiBaseUrl: normalizedApiBaseUrl
	};
	setApiBaseUrl(normalizedApiBaseUrl);
	authSession.set({ ...storedSession, token: session.token });
	if (browser) {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(storedSession));
		localStorage.setItem(API_BASE_STORAGE_KEY, normalizedApiBaseUrl);
		if (Capacitor.isNativePlatform()) {
			await Preferences.set({ key: NATIVE_STORAGE_KEY, value: JSON.stringify({ ...session, apiBaseUrl: normalizedApiBaseUrl }) });
		}
	}
}

export function clearAuthSession() {
	authSession.set(null);
	currentApiBaseUrl = '';
	if (browser) {
		localStorage.removeItem(STORAGE_KEY);
		localStorage.removeItem(API_BASE_STORAGE_KEY);
		if (Capacitor.isNativePlatform()) void Preferences.remove({ key: NATIVE_STORAGE_KEY });
	}
}

export function getAuthToken(): string | null {
	return currentSession?.token ?? null;
}

export function getApiBaseUrl(): string {
	return currentApiBaseUrl;
}

export function setApiBaseUrl(value: string) {
	currentApiBaseUrl = value ? normalizeApiBaseUrl(value) : '';
}

export function normalizeApiBaseUrl(value: string): string {
	const trimmed = value.trim().replace(/\/+$/, '');
	if (!apiBasePattern.test(trimmed)) throw new Error('Use a valid http(s) backend URL without a path.');
	const parsed = new URL(trimmed);
	if (parsed.username || parsed.password || parsed.pathname !== '/' || parsed.search || parsed.hash) {
		throw new Error('Use a backend origin only, without credentials, path, query, or hash.');
	}
	const port = Number(parsed.port || (parsed.protocol === 'https:' ? 443 : 80));
	if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error('Use a valid backend port.');
	return parsed.origin;
}

export async function verifyApiBaseUrl(value: string): Promise<string> {
	const normalized = normalizeApiBaseUrl(value);
	const response = await fetch(`${normalized}/api/health`, { credentials: 'include' });
	if (!response.ok) throw new Error(`Health check failed with status ${response.status}.`);
	const payload = (await response.json().catch(() => null)) as { ok?: boolean } | null;
	if (!payload?.ok) throw new Error('Backend health check did not return an ok response.');
	return normalized;
}

async function readStoredSession(): Promise<string | null> {
	if (Capacitor.isNativePlatform()) {
		const nativeSession = (await Preferences.get({ key: NATIVE_STORAGE_KEY })).value;
		if (nativeSession) return nativeSession;
	}
	return localStorage.getItem(STORAGE_KEY);
}

function readStoredApiBase(): string | null {
	if (!browser) return null;
	try {
		const value = localStorage.getItem(API_BASE_STORAGE_KEY);
		return value ? normalizeApiBaseUrl(value) : null;
	} catch {
		localStorage.removeItem(API_BASE_STORAGE_KEY);
		return null;
	}
}

export function withStreamToken(url: string, token: string): string {
	if (!token) return url;
	const separator = url.includes('?') ? '&' : '?';
	return `${url}${separator}stream_token=${encodeURIComponent(token)}`;
}
