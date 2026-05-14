import { browser } from '$app/environment';
import { Network, type ConnectionStatus } from '@capacitor/network';
import { writable } from 'svelte/store';

export type NetworkType = ConnectionStatus['connectionType'];

const initialStatus: ConnectionStatus = {
	connected: true,
	connectionType: 'unknown'
};

export const networkStatus = writable<ConnectionStatus>(initialStatus);

let initialized = false;

export async function initNetworkStatus() {
	if (!browser || initialized) return;
	initialized = true;

	try {
		networkStatus.set(await Network.getStatus());
		await Network.addListener('networkStatusChange', (status) => {
			networkStatus.set(status);
		});
	} catch (error) {
		console.warn('Unable to initialize Capacitor network status', error);
	}
}

export async function currentNetworkType(): Promise<NetworkType> {
	if (!browser) return 'unknown';

	try {
		const status = await Network.getStatus();
		networkStatus.set(status);
		return status.connectionType;
	} catch (error) {
		console.warn('Unable to read Capacitor network status', error);
		return 'unknown';
	}
}
