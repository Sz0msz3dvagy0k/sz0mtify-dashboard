import { browser } from '$app/environment';
import { writable } from 'svelte/store';

export type ThemeMode = 'dark' | 'light';
export type ThemePalette = 'monochrome' | 'ocean' | 'forest' | 'rose';

export type ThemeSettings = {
	mode: ThemeMode;
	palette: ThemePalette;
};

const storageKey = 'archive.theme.v1';
const modes = new Set<ThemeMode>(['dark', 'light']);
const palettes = new Set<ThemePalette>(['monochrome', 'ocean', 'forest', 'rose']);
const defaultTheme: ThemeSettings = { mode: 'dark', palette: 'monochrome' };

export const themeSettings = writable<ThemeSettings>(defaultTheme);

export function initTheme() {
	if (!browser) return;
	themeSettings.set(readTheme());
	themeSettings.subscribe((settings) => {
		applyTheme(settings);
		localStorage.setItem(storageKey, JSON.stringify(settings));
	});
}

export function updateTheme(next: Partial<ThemeSettings>) {
	themeSettings.update((current) => ({ ...current, ...next }));
}

function readTheme(): ThemeSettings {
	if (!browser) return defaultTheme;
	try {
		const raw = localStorage.getItem(storageKey);
		const parsed = raw ? (JSON.parse(raw) as Partial<ThemeSettings>) : {};
		return {
			mode: parsed.mode && modes.has(parsed.mode) ? parsed.mode : defaultTheme.mode,
			palette: parsed.palette && palettes.has(parsed.palette) ? parsed.palette : defaultTheme.palette
		};
	} catch {
		return defaultTheme;
	}
}

function applyTheme(settings: ThemeSettings) {
	const root = document.documentElement;
	root.dataset.theme = settings.mode;
	root.dataset.palette = settings.palette;
}
