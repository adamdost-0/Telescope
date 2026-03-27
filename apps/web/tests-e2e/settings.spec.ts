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
  await expect(page.getByText(/Saved/)).toBeVisible();

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
