import type { CapacitorConfig } from '@capacitor/cli';

const config: CapacitorConfig = {
	appId: 'me.szomszed.kaori',
	appName: 'Archive',
	webDir: 'build',
	server: {
		iosScheme: 'capacitor'
	}
};

export default config;
