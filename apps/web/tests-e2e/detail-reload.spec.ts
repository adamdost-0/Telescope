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
