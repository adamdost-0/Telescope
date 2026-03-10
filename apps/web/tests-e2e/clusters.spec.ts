import { test, expect } from '@playwright/test';

test('clusters: selecting cluster c1 navigates to /explore?cluster=c1', async ({ page }) => {
  await page.goto('/clusters');

  await expect(page.getByRole('heading', { name: 'Clusters' })).toBeVisible();
  await expect(page.getByRole('button', { name: /Cluster One \(c1\)/ })).toBeVisible();

  await page.getByRole('button', { name: /Cluster One \(c1\)/ }).click();

  await expect(page).toHaveURL(/\/explore\?cluster=c1$/);
  await expect(page.getByRole('heading', { name: 'Explore' })).toBeVisible();
  await expect(page.getByTestId('selected-cluster')).toHaveText('c1');
});
