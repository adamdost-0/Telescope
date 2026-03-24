import { expect, test } from '@playwright/test';
import { installMockTauri } from './helpers/mock-tauri';

test.beforeEach(async ({ page }) => {
  await installMockTauri(page);
});

test('home page renders without errors when connected', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (err) => errors.push(err.message));

  await page.goto('/');
  await expect(page.locator('.app-shell')).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId('app-brand')).toBeVisible();
  expect(errors).toEqual([]);
});

test('pods page renders pod list from mock data', async ({ page }) => {
  await page.goto('/pods');
  // The mock provides nginx-abc123 and redis-def456 pods
  await expect(page.locator('text=nginx-abc123')).toBeVisible();
  await expect(page.locator('text=redis-def456')).toBeVisible();
});

test('pods page shows no page errors on rapid navigation', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (err) => errors.push(err.message));

  await page.goto('/pods');
  await page.goto('/nodes');
  await page.goto('/pods');

  // Allow time for async operations
  await page.waitForTimeout(500);
  expect(errors).toEqual([]);
});
