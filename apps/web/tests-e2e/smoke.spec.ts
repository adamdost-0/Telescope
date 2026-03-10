import { test, expect } from '@playwright/test';

test('smoke: loads home page', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Telescope' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Clusters' })).toBeVisible();
});
