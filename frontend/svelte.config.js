import adapter from '@sveltejs/adapter-node';

export default {
	kit: {
		adapter: adapter(),
		env: {
			dir: '..',
			publicPrefix: 'FRONTEND_'
		}
	}
};
