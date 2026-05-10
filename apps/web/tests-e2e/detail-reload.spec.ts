import { expect, test } from '@playwright/test';
import { installMockTauri } from './helpers/mock-tauri';

test.beforeEach(async ({ page }) => {
  await installMockTauri(page);
});

test('pod detail reloads when navigating between pods', async ({ page }) => {
  await page.goto('/pods/default/nginx-abc123');
  await expect(page.locator('.app-shell')).toBeVisible({ timeout: 15000 });
  await expect(page.getByRole('heading', { name: 'nginx-abc123' })).toBeVisible();

  // Client-side navigate to a different pod
  await page.goto('/pods/default/redis-def456');
  await expect(page.getByRole('heading', { name: 'redis-def456' })).toBeVisible();
});

test('node detail renders correctly from route params', async ({ page }) => {
  await page.goto('/nodes/node-1');
  await expect(page.locator('.app-shell')).toBeVisible({ timeout: 15000 });
  await expect(page.getByRole('heading', { name: 'node-1' })).toBeVisible();
});

test('helm detail renders correctly from route params', async ({ page }) => {
  await page.goto('/helm/default/ingress-nginx');
  await expect(page.locator('.app-shell')).toBeVisible({ timeout: 15000 });
  await expect(page.getByRole('heading', { name: 'ingress-nginx' })).toBeVisible();
});

test('helm detail does not expose Helm values reveal UI and rejects reveal IPC in tests', async ({ page }) => {
  await page.goto('/helm/default/ingress-nginx');
  await expect(page.locator('.app-shell')).toBeVisible({ timeout: 15000 });
  await expect(page.getByRole('heading', { name: 'ingress-nginx' })).toBeVisible();

  await page.getByRole('tab', { name: 'Values' }).click();
  await expect(page.getByTestId('reveal-sensitive-values')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /reveal sensitive values/i })).toHaveCount(0);

  await expect(page.evaluate(() => {
    const tauri = (window as Window & { __TEST_TAURI__: { calls: Array<{ cmd: string; args: Record<string, unknown> }> } }).__TEST_TAURI__;
    return tauri.calls.some((entry) => entry.cmd === 'get_helm_release_values' && entry.args.reveal === true);
  })).resolves.toBe(false);

  await expect(page.evaluate(async () => {
    const tauri = (window as Window & {
      __TAURI_INTERNALS__: {
        invoke: (cmd: string, args: Record<string, unknown>) => Promise<unknown>;
      };
    }).__TAURI_INTERNALS__;

    try {
      await tauri.invoke('get_helm_release_values', {
        namespace: 'default',
        name: 'ingress-nginx',
        reveal: true,
      });
      return 'resolved';
    } catch (error) {
      return error instanceof Error ? error.message : String(error);
    }
  })).resolves.toContain('Revealing raw Helm values is disabled for security.');
});
