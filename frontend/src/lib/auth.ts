import { browser } from '$app/environment';
import { Capacitor } from '@capacitor/core';
import { Preferences } from '@capacitor/preferences';
import { writable } from 'svelte/store';
import type { AuthSession } from '$lib/types';

const STORAGE_KEY = 'music-dashboard.auth';
const NATIVE_STORAGE_KEY = 'music-dashboard.native-auth';

let currentSession: AuthSession | null = null;
export const authSession = writable<AuthSession | null>(null);

authSession.subscribe((value) => {
	currentSession = value;
});

export async function loadStoredSession(): Promise<AuthSession | null> {
	if (!browser) return null;

	try {
		const raw = await readStoredSession();
		if (!raw) return null;
		const session = JSON.parse(raw) as AuthSession;
		if (!session.username || session.expires_at <= Math.floor(Date.now() / 1000)) {
			clearAuthSession();
			return null;
		}
		authSession.set(session);
		return session;
	} catch {
		clearAuthSession();
		return null;
	}
}

export async function saveAuthSession(session: AuthSession) {
	const storedSession = {
		username: session.username,
		expires_at: session.expires_at
	};
	authSession.set({ ...storedSession, token: session.token });
	if (browser) {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(storedSession));
		if (Capacitor.isNativePlatform()) {
			await Preferences.set({ key: NATIVE_STORAGE_KEY, value: JSON.stringify(session) });
		}
	}
}

export function clearAuthSession() {
	authSession.set(null);
	if (browser) {
		localStorage.removeItem(STORAGE_KEY);
		if (Capacitor.isNativePlatform()) void Preferences.remove({ key: NATIVE_STORAGE_KEY });
	}
}

export function getAuthToken(): string | null {
	return currentSession?.token ?? null;
}

async function readStoredSession(): Promise<string | null> {
	if (Capacitor.isNativePlatform()) {
		const nativeSession = (await Preferences.get({ key: NATIVE_STORAGE_KEY })).value;
		if (nativeSession) return nativeSession;
	}
	return localStorage.getItem(STORAGE_KEY);
}

export function withStreamToken(url: string, token: string): string {
	if (!token) return url;
	const separator = url.includes('?') ? '&' : '?';
	return `${url}${separator}stream_token=${encodeURIComponent(token)}`;
}
