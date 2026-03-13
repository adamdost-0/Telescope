import { defineConfig } from '@playwright/test';

const webPort = Number(process.env.PLAYWRIGHT_WEB_PORT ?? '4273');

export default defineConfig({
  testDir: './tests-e2e',
  use: {
    baseURL: `http://127.0.0.1:${webPort}`
  },
  webServer: {
    command: `pnpm exec vite dev --host 127.0.0.1 --port ${webPort} --strictPort`,
    url: `http://127.0.0.1:${webPort}`,
    timeout: 180_000,
    reuseExistingServer: false
  }
});
