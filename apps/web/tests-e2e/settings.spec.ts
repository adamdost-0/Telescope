import { expect, test } from '@playwright/test';
import { version } from '../src/lib/version';

test('settings: about shows the shared app version', async ({ page }) => {
  await page.goto('/settings');

  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
  await expect(page.locator('section.about')).toContainText(version);
});
