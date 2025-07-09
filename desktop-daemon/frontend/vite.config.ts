import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		proxy: {
			'/search': 'http://localhost:3000',
			'/documents': 'http://localhost:3000'
		}
	}
});