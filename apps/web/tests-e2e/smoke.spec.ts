import { test, expect } from '@playwright/test';

test('smoke: loads home page', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Telescope' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Clusters' })).toBeVisible();
});

// Playwright runs the shared frontend in a plain browser session. Until the suite has a
// Tauri IPC/context fixture, E2E can only assert the default non-AKS state here.
test('sidebar: Azure section hidden when no AKS context is available', async ({ page }) => {
  await page.goto('/pods');
  const navigation = page.getByRole('navigation', { name: 'Resource navigation' });
  await expect(navigation.getByText('Azure')).toHaveCount(0);
});
