import { test, expect } from '@playwright/test';

test('clusters: selecting a cluster navigates to /pods', async ({ page }) => {
  await page.goto('/clusters');

  await expect(page.getByRole('heading', { name: 'Clusters' })).toBeVisible();

  const localDev = page.getByRole('button', { name: /Local Dev Cluster/ });
  await expect(localDev).toBeVisible();

  await localDev.click();

  await expect(page).toHaveURL(/\/pods$/);
  await expect(page.getByRole('heading', { name: 'Pods' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'telescope-api-7f6c9d4b7b-abcde' })).toBeVisible();
});
