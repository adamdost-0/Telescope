import { expect, test } from '@playwright/test';
import type { Page } from '@playwright/test';
import { installMockTauri } from './helpers/mock-tauri';

async function gotoNodePools(page: Page) {
  await page.goto('/azure/node-pools');
  await expect(page.getByTestId('node-pools-heading')).toBeVisible({ timeout: 10_000 });
  await expect(page.getByRole('heading', { name: /Node Pools/i })).toBeVisible({ timeout: 10_000 });
  await expect(page.getByText('2 node pools')).toBeVisible();
}

test('node pools: renders AKS data, filters rows, and surfaces upgrade actions', async ({ page }) => {
  await installMockTauri(page);
  await gotoNodePools(page);

  await expect(page.getByRole('button', { name: '+ Create Pool' })).toBeEnabled();
  await expect(page.getByRole('navigation', { name: 'Resource navigation' }).getByText('Node Pools')).toBeVisible();

  await page.getByLabel('Filter resources').fill('gpu');
  await expect(page.getByText('1 of 2 node pools')).toBeVisible();
  await expect(page.getByRole('link', { name: /gpunp/ })).toBeVisible();
  await expect(page.getByRole('link', { name: /systempool/ })).toHaveCount(0);

  await page.getByLabel('Filter resources').fill('');
  await page.getByRole('link', { name: /gpunp/ }).click();

  await expect(page.getByRole('heading', { name: 'Upgrade Options' })).toBeVisible();
  await expect(page.getByText('Latest node image: AKSUbuntu-2204gen2containerd-2024.10.15')).toBeVisible();

  await page.getByRole('button', { name: '1.30.0 Preview' }).click();
  const upgradeDialog = page.getByRole('dialog', { name: 'Upgrade Node Pool' });
  await expect(upgradeDialog).toBeVisible();
  await expect(upgradeDialog.getByRole('button', { name: 'Confirm Upgrade' })).toBeDisabled();
  await upgradeDialog.getByLabel('I understand this targets a preview Kubernetes version.').check();
  await upgradeDialog.getByRole('button', { name: 'Confirm Upgrade' }).click();

  await expect(page.getByText('Pool upgraded to Kubernetes 1.30.0.')).toBeVisible();
  await expect(page.locator('tbody')).toContainText('1.30.0');
});

test('node pools: create, scale, autoscale, and delete pool with correct payloads', async ({ page }) => {
  await installMockTauri(page);
  await gotoNodePools(page);

  const systemRow = page.locator('tbody tr').filter({ hasText: 'systempool' }).first();
  await expect(systemRow.getByRole('button', { name: 'Delete', exact: true })).toBeDisabled();

  await page.getByRole('button', { name: '+ Create Pool' }).click();
  const createDialog = page.getByRole('dialog', { name: 'Create Node Pool' });
  await createDialog.getByPlaceholder('mypool').fill('burstpool');
  await createDialog.getByLabel('Enable autoscaler').check();
  await createDialog.getByLabel('Min count').fill('2');
  await createDialog.getByLabel('Max count').fill('6');
  await createDialog.getByPlaceholder('1,2,3').fill('1, 2');
  await createDialog.getByPlaceholder('env=staging,team=platform').fill('env=staging,team=platform');
  await createDialog.getByPlaceholder('gpu=true:NoSchedule').fill('workload=batch:NoSchedule');
  await createDialog.getByRole('button', { name: 'Create Pool' }).click();

  await expect(page.getByText('Creating node pool "burstpool"…')).toBeVisible();
  await expect(page.getByText('3 node pools')).toBeVisible();
  await expect(page.locator('tbody')).toContainText('burstpool');

  const createCall = await page.evaluate(() => {
    return (window as any).__TEST_TAURI__.calls.findLast((entry: { cmd: string }) => entry.cmd === 'create_aks_node_pool');
  });
  expect(createCall.args.config).toMatchObject({
    name: 'burstpool',
    enableAutoScaling: true,
    minCount: 2,
    maxCount: 6,
    availabilityZones: ['1', '2'],
    nodeLabels: { env: 'staging', team: 'platform' },
    nodeTaints: ['workload=batch:NoSchedule'],
  });

  const burstRow = page.locator('tbody tr').filter({ hasText: 'burstpool' }).first();
  await burstRow.getByTitle('Scale node count').click();
  const scaleDialog = page.getByRole('dialog', { name: 'Scale Node Pool' });
  await scaleDialog.getByRole('spinbutton').fill('4');
  await scaleDialog.getByRole('button', { name: 'Scale' }).click();
  await expect(page.getByText('Scaling burstpool to 4 nodes…')).toBeVisible();
  await expect(page.locator('tbody tr').filter({ hasText: 'burstpool' }).first()).toContainText('4');

  await burstRow.getByTitle('Configure autoscaler').click();
  const autoscalerDialog = page.getByRole('dialog', { name: 'Configure Autoscaler' });
  await autoscalerDialog.getByLabel('Min count').fill('3');
  await autoscalerDialog.getByLabel('Max count').fill('7');
  await autoscalerDialog.getByRole('button', { name: 'Apply' }).click();
  await expect(page.getByText('Autoscaler enabled on burstpool (3–7)')).toBeVisible();
  await expect(page.locator('tbody tr').filter({ hasText: 'burstpool' }).first()).toContainText('3-7');

  await burstRow.getByRole('button', { name: 'Delete', exact: true }).click();
  const deleteDialog = page.getByRole('dialog', { name: 'Delete Node Pool' });
  await deleteDialog.getByRole('textbox').fill('burstpool');
  await deleteDialog.getByRole('button', { name: 'Delete Pool' }).click();

  await expect(page.getByText('Deleting node pool "burstpool"…')).toBeVisible();
  await expect(page.locator('tbody')).not.toContainText('burstpool');
});

test('node pools: ARM load errors are user-friendly and dismissible', async ({ page }) => {
  await installMockTauri(page, {
    commandErrors: {
      list_aks_node_pools: 'Your Azure session has expired. Sign in again and retry.',
    },
  });

  await page.goto('/azure/node-pools');
  await expect(page.getByRole('heading', { name: 'Node Pools' })).toBeVisible();

  const armBanner = page.locator('.arm-error-banner[role="alert"]');
  await expect(armBanner).toBeVisible();
  await expect(armBanner).toContainText('Azure ARM request failed');
  await expect(armBanner).toContainText('Unable to load AKS node pools.');

  await armBanner.getByRole('button', { name: 'Dismiss' }).click();
  await expect(armBanner).toHaveCount(0);
});

test('node pools: retry recovers after transient ARM load error', async ({ page }) => {
  await installMockTauri(page, {
    commandErrors: {
      list_aks_node_pools: {
        message: 'Azure API error (403): [AuthorizationFailed] deny',
        times: 1,
      },
    },
  });

  await page.goto('/azure/node-pools');
  await expect(page.getByRole('heading', { name: 'Node Pools' })).toBeVisible();

  const errorBanner = page.locator('.error-card[role="alert"]');
  await expect(errorBanner).toBeVisible();
  await expect(errorBanner).toContainText('Failed to load node pools from Azure ARM.');

  await errorBanner.getByRole('button', { name: 'Retry' }).click();
  await expect(page.getByText('2 node pools')).toBeVisible();
  await expect(errorBanner).toHaveCount(0);
});
