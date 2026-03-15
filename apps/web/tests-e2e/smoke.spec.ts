import { test, expect } from '@playwright/test';

test('smoke: loads home page', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Telescope' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Clusters' })).toBeVisible();
});

test('sidebar: Azure section hidden on non-AKS clusters', async ({ page }) => {
  await page.goto('/pods');
  const navigation = page.getByRole('navigation', { name: 'Resource navigation' });
  await expect(navigation.getByText('Azure')).toHaveCount(0);
});

test('sidebar: Azure section visible and ordered for AKS clusters', async ({ page }) => {
  await page.goto('/pods');
  await page.getByLabel('Context:').selectOption('Staging AKS');
  await expect(page.getByLabel('Context:')).toHaveValue('Staging AKS');

  const navigation = page.getByRole('navigation', { name: 'Resource navigation' });
  await expect(navigation.getByText('Azure')).toBeVisible();
  await expect(navigation.locator('a[href="/nodes"]', { hasText: 'Node Pools' })).toBeVisible();
  await expect(navigation.locator('a[href="/overview"]', { hasText: 'AKS Add-ons' })).toBeVisible();

  const sectionTitles = await navigation.locator('.section-title').allTextContents();
  expect(sectionTitles.slice(0, 3)).toEqual(['Cluster', 'Azure', 'Workloads']);

  const portalLink = navigation.locator('a', { hasText: 'Portal' });
  await expect(portalLink).toHaveClass(/disabled/);
  await expect(portalLink).toHaveAttribute('aria-disabled', 'true');
  await expect(portalLink).toHaveAttribute('target', '_blank');
});
