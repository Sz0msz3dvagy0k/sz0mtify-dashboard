import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

const allowedHosts = [
	'dashboard.szomszed.me',
	...(process.env.FRONTEND_ALLOWED_HOSTS ?? '')
		.split(',')
		.map((host) => host.trim())
		.filter(Boolean)
];

export default defineConfig({
	envDir: '..',
	envPrefix: ['VITE_', 'FRONTEND_'],
	plugins: [sveltekit()],
	server: {
		allowedHosts
	}
});
