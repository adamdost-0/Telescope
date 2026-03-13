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

async function openSearchPalette(page: Page) {
  const input = page.getByRole('textbox', { name: 'Search resources' });
  await page.locator('body').click();
  await page.keyboard.press('Control+k');
  if (!(await input.isVisible())) {
    await page.evaluate(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true }));
    });
  }
  await expect(input).toBeVisible({ timeout: 10000 });
}

test('sidebar exposes the new workload and batch pages', async ({ page }) => {
  await page.goto('/pods');
  await expect(page.getByRole('heading', { name: 'Pods' })).toBeVisible();

  const navigation = page.getByRole('navigation', { name: 'Resource navigation' });

  await expect(navigation.locator('a[href="/resources/statefulsets"]')).toBeVisible();
  await expect(navigation.locator('a[href="/resources/daemonsets"]')).toBeVisible();
  await expect(navigation.locator('a[href="/resources/jobs"]')).toBeVisible();
  await expect(navigation.locator('a[href="/resources/cronjobs"]')).toBeVisible();
  await expect(navigation.locator('a[href="/resources/ingresses"]')).toBeVisible();
  await expect(navigation.locator('a[href="/resources/pvcs"]')).toBeVisible();

  await openSidebarResource(page, 'StatefulSets', '/resources/statefulsets');
  await openSidebarResource(page, 'DaemonSets', '/resources/daemonsets');
  await openSidebarResource(page, 'Jobs', '/resources/jobs');
  await openSidebarResource(page, 'CronJobs', '/resources/cronjobs');
  await openSidebarResource(page, 'Ingresses', '/resources/ingresses');
  await openSidebarResource(page, 'Persistent Volume Claims', '/resources/pvcs');
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

test('secrets: list and detail use the dedicated on-demand path with redacted values', async ({ page }) => {
  const secretListResponse = page.waitForResponse((response) =>
    response.request().method() === 'GET'
      && response.url().includes('/api/v1/secrets?namespace=default')
  );
  const cachedSecretsResponse = page.waitForResponse((response) =>
    response.request().method() === 'GET'
      && response.url().includes('/api/v1/resources?gvk=v1%2FSecret')
  , { timeout: 1500 }).catch(() => null);

  await page.goto('/resources/secrets');
  await expect(page.getByRole('heading', { name: 'Secrets' })).toBeVisible();

  await secretListResponse;
  expect(await cachedSecretsResponse).toBeNull();
  await expect(page.getByText('1 secrets')).toBeVisible();

  const row = page.locator('tbody tr').filter({ hasText: 'payments-api-secrets' });
  await expectRowValues(row, ['payments-api-secrets', 'Opaque', '3']);

  const detailResponse = page.waitForResponse((response) =>
    response.request().method() === 'GET'
      && response.url().includes('/api/v1/secrets/default/payments-api-secrets')
  );

  await row.getByRole('link', { name: 'payments-api-secrets' }).click();
  await detailResponse;

  await expect(page).toHaveURL(/\/resources\/secrets\/default\/payments-api-secrets$/);
  await expect(page.getByRole('heading', { name: 'payments-api-secrets' })).toBeVisible();
  await expect(page.getByText('Type')).toBeVisible();
  await expect(page.getByText('Opaque')).toBeVisible();
  await expect(page.getByRole('cell', { name: 'username' })).toBeVisible();
  await expect(page.getByText('••••••••')).toHaveCount(3);

  await page.getByRole('tab', { name: 'YAML' }).click();
  const editor = page.getByLabel('YAML editor');
  await expect(editor).toHaveValue(/"password": "●●●●●●●●"/);
  await expect(editor).toHaveValue(/"token": "●●●●●●●●"/);
  await expect(editor).not.toHaveValue(/super-secret|raw-token|plain-text/);
  await expect(page.getByText('editing is disabled to avoid applying masked data', { exact: false })).toBeVisible();
});

test('secrets: namespace switching reloads uncached secrets for the selected namespace', async ({ page }) => {
  await page.goto('/resources/secrets');
  await expect(page.getByRole('heading', { name: 'Secrets' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'payments-api-secrets' })).toBeVisible();

  const secretListResponse = page.waitForResponse((response) =>
    response.request().method() === 'GET'
      && response.url().includes('/api/v1/secrets?namespace=kube-system')
  );

  await page.getByLabel('Namespace:').selectOption('kube-system');
  await secretListResponse;

  await expect(page.getByText('Namespace: kube-system')).toBeVisible();
  await expect(page.getByRole('link', { name: 'azure-provider-tls' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'payments-api-secrets' })).toHaveCount(0);
});

test('ingresses: lists routes and opens ingress details', async ({ page }) => {
  await page.goto('/pods');
  await openSidebarResource(page, 'Ingresses', '/resources/ingresses');

  await expect(page.getByText('2 ingresses')).toBeVisible();

  const checkoutRow = page.locator('tbody tr').filter({ hasText: 'checkout-edge' });
  await expectRowValues(checkoutRow, [
    'checkout-edge',
    'nginx',
    'shop.telescope.dev',
    '20.51.10.12',
    '1'
  ]);

  await checkoutRow.getByRole('link', { name: 'checkout-edge' }).click();
  await expect(page).toHaveURL(/\/resources\/ingresses\/default\/checkout-edge$/);
  await expect(page.getByRole('heading', { name: 'checkout-edge' })).toBeVisible();
  await expect(page.getByText('Ingress Class')).toBeVisible();
  await expect(page.getByText('checkout-edge-tls')).toBeVisible();
  await expect(page.getByText('checkout-api:80')).toBeVisible();
});

test('pvcs: lists claims and opens pvc details', async ({ page }) => {
  await page.goto('/pods');
  await openSidebarResource(page, 'Persistent Volume Claims', '/resources/pvcs');

  await expect(page.getByText('2 persistent volume claims')).toBeVisible();

  const ordersPvcRow = page.locator('tbody tr').filter({ hasText: 'orders-db-data-0' });
  await expectRowValues(ordersPvcRow, [
    'orders-db-data-0',
    'Bound',
    'pvc-0f1e2d3c4b5a',
    '128Gi',
    'managed-csi-premium'
  ]);

  await ordersPvcRow.getByRole('link', { name: 'orders-db-data-0' }).click();
  await expect(page).toHaveURL(/\/resources\/pvcs\/default\/orders-db-data-0$/);
  await expect(page.getByRole('heading', { name: 'orders-db-data-0' })).toBeVisible();
  await expect(page.getByText('Storage Class')).toBeVisible();
  await expect(page.getByText('managed-csi-premium')).toBeVisible();
  await expect(page.getByText('ReadWriteOnce')).toBeVisible();
});

test('search palette routes ingresses and pvcs to plural detail pages', async ({ page }) => {
  await page.goto('/pods');

  await openSearchPalette(page);
  await page.getByRole('textbox', { name: 'Search resources' }).fill('checkout-edge');
  const ingressOption = page.getByRole('option', { name: /checkout-edge/ });
  await expect(ingressOption).toBeVisible();
  await ingressOption.click();
  await expect(page).toHaveURL(/\/resources\/ingresses\/default\/checkout-edge$/);

  await openSearchPalette(page);
  await page.getByRole('textbox', { name: 'Search resources' }).fill('orders-db-data-0');
  const pvcOption = page.getByRole('option', { name: /orders-db-data-0/ });
  await expect(pvcOption).toBeVisible();
  await pvcOption.click();
  await expect(page).toHaveURL(/\/resources\/pvcs\/default\/orders-db-data-0$/);
});
