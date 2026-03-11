import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  build: {
    rollupOptions: {
      // @tauri-apps/api is only available at runtime in the desktop app
      external: ['@tauri-apps/api/core', '@tauri-apps/api/event']
    }
  },
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
