import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    proxy: process.env.PUBLIC_ENGINE_HTTP_BASE
      ? { '/api': { target: process.env.PUBLIC_ENGINE_HTTP_BASE, changeOrigin: true } }
      : undefined
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
    exclude: ['tests-e2e/**', 'node_modules/**']
  }
});
