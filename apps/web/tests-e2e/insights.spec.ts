import { expect, test } from '@playwright/test';
import { installMockTauri } from './helpers/mock-tauri';

const paletteDialog = '[role="dialog"]';

async function openPalette(page: import('@playwright/test').Page) {
  await expect(page.locator('.app-shell')).toBeVisible({ timeout: 15000 });
  await page.evaluate(() => {
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true }));
  });
  await expect(page.locator(paletteDialog)).toBeVisible();
}

test('insights: connected flow supports test, generate, and clear history', async ({ page }) => {
  await installMockTauri(page);
  await page.goto('/insights');

  await expect(page.getByRole('heading', { name: 'AI Insights' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'History' })).toBeVisible();
  await expect(page.getByLabel('API key (session only)')).toBeHidden();

  await page.getByRole('button', { name: 'Test connection' }).click();
  await expect(page.getByText('AI Insights connection test succeeded.')).toBeVisible();
  // Check that endpoint is shown in the connection result (first dd with example.openai.azure.com)
  await expect(page.locator('dd:has-text("example.openai.azure.com")').first()).toBeVisible();

  await page.getByRole('button', { name: 'Generate insights' }).click();
  await expect(page.getByText('Generated AI Insights.')).toBeVisible();

  const commandNames = await page.evaluate(() =>
    (window as unknown as { __TEST_TAURI__: { calls: Array<{ cmd: string }> } })
      .__TEST_TAURI__
      .calls
      .map((call) => call.cmd),
  );
  expect(commandNames).toContain('test_ai_insights_connection');
  expect(commandNames).toContain('generate_ai_insights');
  expect(commandNames).toContain('list_ai_insights_history');

  const nonApiKeyCallArgs = await page.evaluate(() => {
    const tauri = (window as unknown as {
      __TEST_TAURI__: { calls: Array<{ cmd: string; args?: { apiKey?: string } }> };
    }).__TEST_TAURI__;
    return tauri.calls
      .filter((call) => call.cmd === 'test_ai_insights_connection' || call.cmd === 'generate_ai_insights')
      .map((call) => ({ cmd: call.cmd, hasApiKey: typeof call.args?.apiKey === 'string' }));
  });
  expect(nonApiKeyCallArgs).toEqual([
    { cmd: 'test_ai_insights_connection', hasApiKey: false },
    { cmd: 'generate_ai_insights', hasApiKey: false },
  ]);

  await page.getByRole('button', { name: 'Clear all' }).click();
  await expect(page.getByText('Cleared AI Insights history.')).toBeVisible();
  await expect(page.getByText('No history entries yet.')).toBeVisible();
});

test('insights: apiKey auth mode passes session key to backend commands only', async ({ page }) => {
  await installMockTauri(page, {
    preferences: {
      ai_insights_auth_mode: 'apiKey',
    },
  });
  await page.goto('/insights');

  await expect(page.getByLabel('API key (session only)')).toBeVisible();
  await page.getByLabel('API key (session only)').fill('session-only-test-key');

  await page.getByRole('button', { name: 'Test connection' }).click();
  await expect(page.getByText('AI Insights connection test succeeded.')).toBeVisible();
  await page.getByRole('button', { name: 'Generate insights' }).click();
  await expect(page.getByText('Generated AI Insights.')).toBeVisible();

  const insightCallArgs = await page.evaluate(() => {
    const tauri = (window as unknown as {
      __TEST_TAURI__: { calls: Array<{ cmd: string; args?: { apiKey?: string; key?: string; value?: string } }> };
    }).__TEST_TAURI__;
    return tauri.calls
      .filter((call) => call.cmd === 'test_ai_insights_connection' || call.cmd === 'generate_ai_insights')
      .map((call) => ({ cmd: call.cmd, apiKey: call.args?.apiKey ?? null }));
  });

  expect(insightCallArgs).toEqual([
    { cmd: 'test_ai_insights_connection', apiKey: 'session-only-test-key' },
    { cmd: 'generate_ai_insights', apiKey: 'session-only-test-key' },
  ]);

  const persistedApiKeyWrites = await page.evaluate(() => {
    const tauri = (window as unknown as {
      __TEST_TAURI__: { calls: Array<{ cmd: string; args?: { key?: string; value?: string } }> };
    }).__TEST_TAURI__;
    return tauri.calls.filter(
      (call) => call.cmd === 'set_preference'
        && (String(call.args?.key ?? '').includes('api_key') || String(call.args?.value ?? '').includes('session-only-test-key')),
    );
  });
  expect(persistedApiKeyWrites).toEqual([]);
});

test('insights: session API key is cleared after route unmount', async ({ page }) => {
  await installMockTauri(page, {
    preferences: {
      ai_insights_auth_mode: 'apiKey',
    },
  });
  await page.goto('/insights');

  const apiKeyInput = page.getByLabel('API key (session only)');
  await expect(apiKeyInput).toBeVisible();
  await apiKeyInput.fill('session-only-route-key');

  await page.goto('/overview');
  await expect(page.getByRole('heading', { name: 'Overview' })).toBeVisible();

  await page.goto('/insights');
  await expect(apiKeyInput).toBeVisible();
  await expect(apiKeyInput).toHaveValue('');

  await page.getByRole('button', { name: 'Test connection' }).click();
  await expect(page.getByText('AI Insights connection test succeeded.')).toBeVisible();

  const latestTestCall = await page.evaluate(() => {
    const tauri = (window as unknown as {
      __TEST_TAURI__: { calls: Array<{ cmd: string; args?: { apiKey?: string } }> };
    }).__TEST_TAURI__;
    const calls = tauri.calls.filter((call) => call.cmd === 'test_ai_insights_connection');
    return calls.at(-1) ?? null;
  });

  expect(latestTestCall).not.toBeNull();
  expect(latestTestCall?.args?.apiKey).toBeUndefined();
});

test('insights: route remains accessible while disconnected and disables generation controls', async ({ page }) => {
  await installMockTauri(page, {
    commandErrors: {
      connect_to_context: 'offline',
    },
  });
  await page.goto('/insights');

  await expect(page.getByRole('heading', { name: 'AI Insights' })).toBeVisible({ timeout: 15000 });
  await expect(page.getByText('Connect to a cluster to test or generate insights. History remains available while disconnected.')).toBeVisible({ timeout: 10000 });
  await expect(page.getByRole('button', { name: 'Test connection' })).toBeDisabled({ timeout: 5000 });
  await expect(page.getByRole('button', { name: 'Generate insights' })).toBeDisabled({ timeout: 5000 });
  await expect(
    page
      .getByRole('complementary', { name: 'AI Insights history' })
      .getByText('Cluster is healthy with one medium risk.')
      .first(),
  ).toBeVisible();
});

test('insights: discoverable from search palette navigation', async ({ page }) => {
  await installMockTauri(page);
  await page.goto('/overview');
  await openPalette(page);

  await page.locator(`${paletteDialog} input`).fill('/ insights');
  await expect(page.getByRole('option', { name: /Insights/ })).toBeVisible();
  await page.getByRole('option', { name: /Insights/ }).first().click();

  await expect(page).toHaveURL(/\/insights/);
  await expect(page.getByRole('heading', { name: 'AI Insights' })).toBeVisible();
});
