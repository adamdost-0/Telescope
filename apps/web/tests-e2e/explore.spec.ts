import { test, expect } from '@playwright/test';

test('explore: banner shows cluster + namespace, kind switch updates list', async ({ page }) => {
  await page.goto('/clusters');
  await page.getByRole('button', { name: /Local Dev Cluster \(local-dev\)/ }).click();

  await expect(page).toHaveURL(/\/explore\?cluster=local-dev/);

  // Banner
  await expect(page.getByTestId('context-banner')).toBeVisible();
  await expect(page.getByTestId('cluster-name')).toContainText('Local Dev Cluster');

  // Namespace default
  await expect(page.getByTestId('namespace-select')).toHaveValue('default');

  // Switch kind
  await page.getByTestId('kind-Nodes').click();
  await expect(page.getByRole('heading', { name: 'Explore' })).toBeVisible();
  await expect(page.getByTestId('resource-table')).toBeVisible();
  await expect(page.getByText('node-1')).toBeVisible();
});
