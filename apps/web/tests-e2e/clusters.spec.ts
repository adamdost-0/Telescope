import { test, expect } from '@playwright/test';

test('clusters: renders clusters returned by stub engine', async ({ page }) => {
  await page.goto('/');

  const items = page.getByTestId('cluster-item');
  await expect(items).toHaveCount(2);
  await expect(page.getByText('Local Dev Cluster')).toBeVisible();
  await expect(page.getByText('Staging AKS')).toBeVisible();
});
