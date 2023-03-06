import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [sveltekit()],
	test: {
		include: ['src/**/*.{test,spec}.{js,ts}']
	},
	server: {
		proxy: {
			'/api': {
				target: 'http://127.0.0.1:4000',
				changeOrigin: true,
			  },
		},
		fs: {
			// Allow serving files from one level up to the project root
			allow: [".."],
		  },
	}
});
