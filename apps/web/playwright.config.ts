import { defineConfig } from '@playwright/test';

const webPort = 4173;
const stubPort = 4174;

export default defineConfig({
  testDir: './tests-e2e',
  use: {
    baseURL: `http://127.0.0.1:${webPort}`
  },
  webServer: [
    {
      command: `STUB_PORT=${stubPort} node ./tests-e2e/stub/stub-server.mjs`,
      url: `http://127.0.0.1:${stubPort}/healthz`,
      reuseExistingServer: !process.env.CI
    },
    {
      command: `ENGINE_HTTP_BASE=http://127.0.0.1:${stubPort} npm run dev -- --host 127.0.0.1 --port ${webPort}`,
      url: `http://127.0.0.1:${webPort}`,
      reuseExistingServer: !process.env.CI
    }
  ]
});
