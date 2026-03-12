import { expect, test, type Locator, type Page } from '@playwright/test';

async function openSidebarResource(page: Page, label: string, path: string) {
  const navigation = page.getByRole('navigation', { name: 'Resource navigation' });
  await navigation.locator(`a[href="${path}"]`).click();
  await expect(page).toHaveURL(new RegExp(`${path}$`));
  await expect(page.getByRole('heading', { name: label })).toBeVisible();
}

async function expectRowValues(row: Locator, expectedTexts: string[]) {
  for (const text of expectedTexts) {
    await expect(row).toContainText(text);
  }
}

test('sidebar exposes the new workload and batch pages', async ({ page }) => {
  await page.goto('/pods');
  await expect(page.getByRole('heading', { name: 'Pods' })).toBeVisible();

  const navigation = page.getByRole('navigation', { name: 'Resource navigation' });

  await expect(navigation.locator('a[href="/resources/statefulsets"]')).toBeVisible();
  await expect(navigation.locator('a[href="/resources/daemonsets"]')).toBeVisible();
  await expect(navigation.locator('a[href="/resources/jobs"]')).toBeVisible();
  await expect(navigation.locator('a[href="/resources/cronjobs"]')).toBeVisible();

  await openSidebarResource(page, 'StatefulSets', '/resources/statefulsets');
  await openSidebarResource(page, 'DaemonSets', '/resources/daemonsets');
  await openSidebarResource(page, 'Jobs', '/resources/jobs');
  await openSidebarResource(page, 'CronJobs', '/resources/cronjobs');
});

test('statefulsets: lists replicated storage workloads and opens details', async ({ page }) => {
  await page.goto('/pods');
  await openSidebarResource(page, 'StatefulSets', '/resources/statefulsets');

  await expect(page.getByText('2 statefulsets')).toBeVisible();

  const ordersDbRow = page.locator('tbody tr').filter({ hasText: 'orders-db' });
  await expectRowValues(ordersDbRow, [
    'orders-db',
    '3/3',
    '3',
    'orders-db-headless',
    'OrderedReady',
    'mcr.microsoft.com/oss/bitnami/postgresql:16.4.0'
  ]);

  const filter = page.getByLabel('Filter resources');
  await filter.fill('orders');
  await expect(page.getByText('1 of 2 statefulsets')).toBeVisible();
  await expect(page.getByRole('link', { name: 'orders-db' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'session-store' })).toHaveCount(0);

  await ordersDbRow.getByRole('link', { name: 'orders-db' }).click();
  await expect(page).toHaveURL(/\/resources\/statefulsets\/default\/orders-db$/);
  await expect(page.getByRole('heading', { name: 'orders-db' })).toBeVisible();
  await expect(page.getByText('Service Name')).toBeVisible();
  await expect(page.getByText('orders-db-headless')).toBeVisible();
  await expect(page.getByText('Volume Claim Templates')).toBeVisible();
  await expect(page.getByText('128Gi')).toBeVisible();
});

test('daemonsets: shows node-scoped agents and rollout actions', async ({ page }) => {
  await page.goto('/pods');
  await openSidebarResource(page, 'DaemonSets', '/resources/daemonsets');

  await expect(page.getByText('2 daemonsets')).toBeVisible();

  const metricsRow = page.locator('tbody tr').filter({ hasText: 'ama-metrics-node' });
  await expectRowValues(metricsRow, [
    'ama-metrics-node',
    '5/5',
    '5',
    'mcr.microsoft.com/azuremonitor/containerinsights/ciprod:3.1.18'
  ]);

  await metricsRow.getByRole('link', { name: 'ama-metrics-node' }).click();
  await expect(page).toHaveURL(/\/resources\/daemonsets\/default\/ama-metrics-node$/);
  await expect(page.getByRole('heading', { name: 'ama-metrics-node' })).toBeVisible();
  await expect(page.getByText('Update Strategy')).toBeVisible();
  await expect(page.getByText('RollingUpdate')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Restart Rollout' })).toBeVisible();
  await expect(page.getByText('app.kubernetes.io/part-of=aks-observability')).toBeVisible();
});

test('jobs: reports completed and failed batch runs', async ({ page }) => {
  await page.goto('/pods');
  await openSidebarResource(page, 'Jobs', '/resources/jobs');

  await expect(page.getByText('2 jobs')).toBeVisible();

  const completedRow = page.locator('tbody tr').filter({ hasText: 'nightly-ledger-close-28903412' });
  await expectRowValues(completedRow, [
    'nightly-ledger-close-28903412',
    'Complete',
    '1/1',
    '0'
  ]);

  const failedRow = page.locator('tbody tr').filter({ hasText: 'sales-export-backfill-20250311' });
  await expectRowValues(failedRow, [
    'sales-export-backfill-20250311',
    'Failed',
    '1/3'
  ]);

  await completedRow.getByRole('link', { name: 'nightly-ledger-close-28903412' }).click();
  await expect(page).toHaveURL(/\/resources\/jobs\/default\/nightly-ledger-close-28903412$/);
  await expect(page.getByRole('heading', { name: 'nightly-ledger-close-28903412' })).toBeVisible();
  await expect(page.getByText('Backoff Limit')).toBeVisible();
  await expect(page.getByText('2025-03-11T02:03:00.000Z')).toBeVisible();
});

test('cronjobs: shows schedules, suspend state, and active runs', async ({ page }) => {
  await page.goto('/pods');
  await openSidebarResource(page, 'CronJobs', '/resources/cronjobs');

  await expect(page.getByText('2 cronjobs')).toBeVisible();

  const nightlyPruneRow = page.locator('tbody tr').filter({ hasText: 'nightly-image-prune' });
  await expectRowValues(nightlyPruneRow, [
    'nightly-image-prune',
    '0 2 * * *',
    'No',
    '1'
  ]);

  const weeklyReportRow = page.locator('tbody tr').filter({ hasText: 'weekly-cost-report' });
  await expectRowValues(weeklyReportRow, [
    'weekly-cost-report',
    '30 6 * * 1',
    'Yes',
    '0'
  ]);

  await nightlyPruneRow.getByRole('link', { name: 'nightly-image-prune' }).click();
  await expect(page).toHaveURL(/\/resources\/cronjobs\/default\/nightly-image-prune$/);
  await expect(page.getByRole('heading', { name: 'nightly-image-prune' })).toBeVisible();
  await expect(page.getByText('Concurrency Policy')).toBeVisible();
  await expect(page.getByText('Forbid')).toBeVisible();
  await expect(page.getByText('nightly-image-prune-28903412')).toBeVisible();
});
