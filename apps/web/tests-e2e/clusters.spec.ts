import { test, expect } from '@playwright/test';

test('clusters: selecting a cluster navigates to /explore?cluster=<id>', async ({ page }) => {
  await page.goto('/clusters');

  await expect(page.getByRole('heading', { name: 'Clusters' })).toBeVisible();

  const localDev = page.getByRole('button', { name: /Local Dev Cluster \(local-dev\)/ });
  await expect(localDev).toBeVisible();

  await localDev.click();

  await expect(page).toHaveURL(/\/explore\?cluster=local-dev$/);
  await expect(page.getByRole('heading', { name: 'Explore' })).toBeVisible();
  await expect(page.getByTestId('selected-cluster')).toHaveText('local-dev');
});
