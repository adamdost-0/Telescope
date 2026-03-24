import { test, expect } from '@playwright/test';
import { installMockTauri } from './helpers/mock-tauri';

test.beforeEach(async ({ page }) => {
  await installMockTauri(page);
});

test('smoke: loads home page', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.app-shell')).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId('app-brand')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Clusters' })).toBeVisible();
});

// Playwright runs the shared frontend in a plain browser session. Until the suite has a
// Tauri IPC/context fixture, E2E can only assert the default non-AKS state here.
test('sidebar: Azure section hidden when no AKS context is available', async ({ page }) => {
  await page.goto('/pods');
  const navigation = page.getByRole('navigation', { name: 'Resource navigation' });
  await expect(navigation.getByText('Azure')).toHaveCount(0);
});

test('header: cluster vitals structure is rendered', async ({ page }) => {
  await page.goto('/overview');

  const vitals = page.locator('header.app-header .cluster-vitals');
  await expect(vitals).toHaveCount(1);
  await expect(vitals).toContainText('CPU');
  await expect(vitals).toContainText('Memory');
});
