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
