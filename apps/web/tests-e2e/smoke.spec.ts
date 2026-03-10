import { test, expect } from '@playwright/test';

test('smoke: loads scaffold page', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Telescope web UI scaffold' })).toBeVisible();
});
