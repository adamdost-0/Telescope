import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
    exclude: ['tests-e2e/**', 'node_modules/**']
  }
});
