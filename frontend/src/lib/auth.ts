import { browser } from '$app/environment';
import { writable } from 'svelte/store';
import type { AuthSession } from '$lib/types';

const STORAGE_KEY = 'music-dashboard.auth';

let currentSession: AuthSession | null = null;
export const authSession = writable<AuthSession | null>(null);

authSession.subscribe((value) => {
	currentSession = value;
});

export function loadStoredSession(): AuthSession | null {
	if (!browser) return null;

	try {
		const raw = localStorage.getItem(STORAGE_KEY);
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

export function saveAuthSession(session: AuthSession) {
	const storedSession = {
		username: session.username,
		expires_at: session.expires_at
	};
	authSession.set({ ...storedSession, token: session.token });
	if (browser) {
		localStorage.setItem(
			STORAGE_KEY,
			JSON.stringify(storedSession)
		);
	}
}

export function clearAuthSession() {
	authSession.set(null);
	if (browser) localStorage.removeItem(STORAGE_KEY);
}

export function getAuthToken(): string | null {
	return currentSession?.token ?? loadStoredSession()?.token ?? null;
}

export function withStreamToken(url: string, token: string): string {
	if (!token) return url;
	const separator = url.includes('?') ? '&' : '?';
	return `${url}${separator}stream_token=${encodeURIComponent(token)}`;
}
