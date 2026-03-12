import { defineConfig } from '@playwright/test';

const webPort = Number(process.env.PLAYWRIGHT_WEB_PORT ?? '4273');
const stubPort = Number(process.env.PLAYWRIGHT_STUB_PORT ?? '4274');

export default defineConfig({
  testDir: './tests-e2e',
  use: {
    baseURL: `http://127.0.0.1:${webPort}`
  },
  webServer: [
    {
      command: `STUB_PORT=${stubPort} node ./tests-e2e/stub/stub-server.mjs`,
      url: `http://127.0.0.1:${stubPort}/healthz`,
      timeout: 30_000,
      reuseExistingServer: false
    },
    {
      command: `PUBLIC_ENGINE_HTTP_BASE=http://127.0.0.1:${stubPort} pnpm exec vite dev --host 127.0.0.1 --port ${webPort} --strictPort`,
      url: `http://127.0.0.1:${webPort}`,
      timeout: 180_000,
      reuseExistingServer: false
    }
  ]
});
