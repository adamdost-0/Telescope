import { expect, test } from '@playwright/test';
import { version } from '../src/lib/version';
import { installMockTauri } from './helpers/mock-tauri';

test('settings: about shows the shared app version', async ({ page }) => {
  await installMockTauri(page);
  await page.goto('/settings');

  await expect(page.locator('.app-shell')).toBeVisible({ timeout: 15000 });
  await expect(page.getByRole('heading', { name: /Settings/i })).toBeVisible();
  await expect(page.locator('section.about')).toContainText(version);
});

test('settings: AKS overrides load and save through scoped IPC for the connected cluster', async ({ page }) => {
  await installMockTauri(page, {
    aksIdentityOverride: {
      isConnected: true,
      isAks: true,
      contextName: 'aks-prod',
      clusterFqdn: 'prod.hcp.eastus.azmk8s.io',
      hasOverride: true,
      subscriptionId: 'sub-saved',
      resourceGroup: 'rg-saved',
      clusterName: 'cluster-saved',
    },
    resolvedAksIdentity: {
      subscription_id: 'sub-detected',
      resource_group: 'rg-detected',
      cluster_name: 'cluster-detected',
      arm_resource_id:
        '/subscriptions/sub-detected/resourceGroups/rg-detected/providers/Microsoft.ContainerService/managedClusters/cluster-detected',
    },
  });
  await page.goto('/settings');

  await expect(page.getByText(/saved only for the currently connected AKS cluster/i)).toBeVisible();
  await expect(page.getByText(/aks-prod/)).toBeVisible();
  await expect(page.getByText(/prod\.hcp\.eastus\.azmk8s\.io/)).toBeVisible();
  await expect(page.getByLabel('Subscription ID')).toHaveValue('sub-saved');
  await expect(page.getByLabel('Resource Group')).toHaveValue('rg-saved');
  await expect(page.getByLabel('Cluster Name')).toHaveValue('cluster-saved');

  await page.getByRole('button', { name: 'Auto-detect from cluster' }).click();
  await expect(page.getByLabel('Subscription ID')).toHaveValue('sub-detected');
  await expect(page.getByLabel('Resource Group')).toHaveValue('rg-detected');
  await expect(page.getByLabel('Cluster Name')).toHaveValue('cluster-detected');

  await page.getByRole('button', { name: 'Save preferences' }).click();
  await expect(page.locator('.saved-badge')).toBeVisible();

  const aksOverrideCalls = await page.evaluate(() => {
    const tauri = (window as Window & {
      __TEST_TAURI__: {
        calls: Array<{
          cmd: string;
          args: { key?: string; preferOverride?: boolean; settings?: Record<string, string> };
        }>;
      };
    }).__TEST_TAURI__;

    return tauri.calls;
  });

  expect(aksOverrideCalls.some((entry) => entry.cmd === 'get_aks_identity_override')).toBe(true);
  expect(aksOverrideCalls.some((entry) =>
    entry.cmd === 'resolve_aks_identity' && entry.args.preferOverride === false
  )).toBe(true);
  expect(aksOverrideCalls).toEqual(expect.arrayContaining([
    expect.objectContaining({
      cmd: 'set_aks_identity_override',
      args: {
        settings: {
          subscriptionId: 'sub-detected',
          resourceGroup: 'rg-detected',
          clusterName: 'cluster-detected',
        },
      },
    }),
  ]));
  expect(aksOverrideCalls.some((entry) =>
    entry.cmd === 'set_preference'
      && ['azure_subscription', 'azure_resource_group', 'azure_cluster_name'].includes(String(entry.args.key))
  )).toBe(false);
});

test('settings: AI Insights non-secret preferences load and save without persisting api keys', async ({ page }) => {
  await installMockTauri(page, {
    preferences: {
      theme: 'dark',
      production_patterns: 'prod',
      default_namespace: 'default',
      auto_refresh_interval: '30',
      ai_insights_endpoint: 'https://initial.openai.azure.com/',
      ai_insights_deployment_name: 'initial-deployment',
      ai_insights_auth_mode: 'apiKey',
      ai_insights_cloud_profile: 'usGovernment',
      ai_insights_model_name: 'gpt-4.1',
    },
  });
  await page.goto('/settings');

  await expect(page.getByRole('heading', { name: /AI Insights/i })).toBeVisible();
  await expect(page.getByText(/API keys are never persisted/i)).toBeVisible();

  await expect(page.getByLabel('Endpoint')).toHaveValue('https://initial.openai.azure.com/');
  await expect(page.getByLabel('Deployment name')).toHaveValue('initial-deployment');
  await expect(page.getByLabel('Authentication mode')).toHaveValue('apiKey');
  await expect(page.getByLabel('Cloud profile')).toHaveValue('usGovernment');
  await expect(page.getByLabel('Model name (optional)')).toHaveValue('gpt-4.1');

  await page.getByLabel('Endpoint').fill('https://saved.openai.azure.com/');
  await page.getByLabel('Deployment name').fill('saved-deployment');
  await page.getByLabel('Authentication mode').selectOption('azureLogin');
  await page.getByLabel('Cloud profile').selectOption('commercial');
  await page.getByLabel('Model name (optional)').fill('');

  await page.getByRole('button', { name: 'Save preferences' }).click();
  await expect(page.locator('.saved-badge')).toBeVisible();

  const aiPreferenceWrites = await page.evaluate(() => {
    const tauri = (window as Window & {
      __TEST_TAURI__: {
        calls: Array<{ cmd: string; args: { key?: string; value?: string } }>;
      };
    }).__TEST_TAURI__;

    return tauri.calls
      .filter((entry) => entry.cmd === 'set_preference'
        && typeof entry.args.key === 'string'
        && entry.args.key.startsWith('ai_insights_'))
      .map((entry) => ({
        key: String(entry.args.key),
        value: String(entry.args.value ?? ''),
      }));
  });

  expect(aiPreferenceWrites).toEqual(expect.arrayContaining([
    { key: 'ai_insights_endpoint', value: 'https://saved.openai.azure.com/' },
    { key: 'ai_insights_deployment_name', value: 'saved-deployment' },
    { key: 'ai_insights_auth_mode', value: 'azureLogin' },
    { key: 'ai_insights_cloud_profile', value: 'commercial' },
    { key: 'ai_insights_model_name', value: '' },
  ]));
  expect(aiPreferenceWrites.some((entry) => entry.key.includes('api_key'))).toBe(false);
});
