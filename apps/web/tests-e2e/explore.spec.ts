import { test, expect } from '@playwright/test';

test('explore: banner shows cluster + namespace, kind switch updates list', async ({ page }) => {
  await page.goto('/clusters');

  await Promise.all([
    page.waitForURL(/\/explore\?cluster=local-dev/),
    page.getByRole('button', { name: /Local Dev Cluster \(local-dev\)/ }).click()
  ]);

  // Banner
  await expect(page.getByTestId('context-banner')).toBeVisible();
  await expect(page.getByTestId('cluster-name')).toContainText('Local Dev Cluster');

  // Namespace default
  await expect(page.getByTestId('namespace-select')).toHaveValue('default');

  // Switch kind
  await Promise.all([
    page.waitForURL(/kind=Nodes/),
    page.getByTestId('kind-Nodes').click()
  ]);

  await expect(page.getByRole('heading', { name: 'Explore' })).toBeVisible();

  const table = page.getByTestId('resource-table');
  await expect(table).toBeVisible();
  await expect(table.getByRole('cell', { name: 'node-1' })).toBeVisible();
  // Nodes are cluster-scoped; namespace column should show '-'
  await expect(table.getByRole('cell', { name: '-' }).first()).toBeVisible();
});
