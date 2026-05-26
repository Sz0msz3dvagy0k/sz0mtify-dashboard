import { browser } from '$app/environment';
import { Capacitor } from '@capacitor/core';
import { Network, type ConnectionStatus } from '@capacitor/network';
import { writable } from 'svelte/store';

export type NetworkType = ConnectionStatus['connectionType'];

const initialStatus: ConnectionStatus = {
	connected: true,
	connectionType: 'unknown'
};

export const networkStatus = writable<ConnectionStatus>(initialStatus);

const STATUS_CACHE_MS = 1000;
let initialized = false;
let cachedStatus: { status: ConnectionStatus; readAt: number } | null = null;

export async function initNetworkStatus() {
	if (!browser || initialized) return;
	initialized = true;

	if (!Capacitor.isNativePlatform()) {
		setNetworkStatus(browserNetworkStatus());
		window.addEventListener('online', updateBrowserNetworkStatus);
		window.addEventListener('offline', updateBrowserNetworkStatus);
		return;
	}

	try {
		setNetworkStatus(await Network.getStatus());
		await Network.addListener('networkStatusChange', (status) => {
			setNetworkStatus(status);
		});
	} catch (error) {
		setNetworkStatus(browserNetworkStatus());
		if (!isUnimplementedError(error)) {
			console.warn('Unable to initialize Capacitor network status', error);
		}
	}
}

export async function currentNetworkType(): Promise<NetworkType> {
	const status = await currentNetworkStatus();
	return status.connectionType;
}

export async function currentNetworkStatus(): Promise<ConnectionStatus> {
	if (!browser) return initialStatus;
	if (cachedStatus && Date.now() - cachedStatus.readAt < STATUS_CACHE_MS) return cachedStatus.status;

	if (!Capacitor.isNativePlatform()) {
		const status = browserNetworkStatus();
		setNetworkStatus(status);
		return status;
	}

	try {
		const status = await Network.getStatus();
		setNetworkStatus(status);
		return status;
	} catch (error) {
		const status = browserNetworkStatus();
		setNetworkStatus(status);
		if (!isUnimplementedError(error)) {
			console.warn('Unable to read Capacitor network status', error);
		}
		return status;
	}
}

export async function isOfflineMode(): Promise<boolean> {
	return !(await currentNetworkStatus()).connected;
}

function updateBrowserNetworkStatus() {
	setNetworkStatus(browserNetworkStatus());
}

function setNetworkStatus(status: ConnectionStatus) {
	cachedStatus = { status, readAt: Date.now() };
	networkStatus.set(status);
}

function browserNetworkStatus(): ConnectionStatus {
	if (!browser) return initialStatus;
	const navigatorWithConnection = navigator as Navigator & {
		connection?: { type?: string; effectiveType?: string };
		mozConnection?: { type?: string; effectiveType?: string };
		webkitConnection?: { type?: string; effectiveType?: string };
	};
	const connection =
		navigatorWithConnection.connection ??
		navigatorWithConnection.mozConnection ??
		navigatorWithConnection.webkitConnection;
	const connectionType = normalizeConnectionType(connection?.type ?? connection?.effectiveType);

	return {
		connected: navigator.onLine,
		connectionType: navigator.onLine ? connectionType : 'none'
	};
}

function normalizeConnectionType(value: string | undefined): NetworkType {
	switch (value) {
		case 'cellular':
		case '2g':
		case '3g':
		case '4g':
		case '5g':
			return 'cellular';
		case 'wifi':
		case 'ethernet':
			return 'wifi';
		case 'none':
			return 'none';
		default:
			return 'unknown';
	}
}

function isUnimplementedError(error: unknown) {
	if (!(error instanceof Error)) return false;
	return /unimplemented|not implemented/i.test(error.message);
}
