import { expect, test } from '@playwright/test';
import { installMockTauri } from './helpers/mock-tauri';

const paletteDialog = '[role="dialog"]';

test.beforeEach(async ({ page }) => {
  await installMockTauri(page);
});

/** Open the search palette by dispatching a keyboard event. */
async function openPalette(page: import('@playwright/test').Page) {
  // Wait for the app layout to be rendered before dispatching the shortcut
  await expect(page.locator('.app-shell')).toBeVisible({ timeout: 15000 });
  await page.evaluate(() => {
    window.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })
    );
  });
  await expect(page.locator(paletteDialog)).toBeVisible();
}

test('search palette: open with Ctrl+K, type "> settings", press Enter → navigates to /settings without errors', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (err) => errors.push(err.message));

  await page.goto('/');
  await openPalette(page);

  await page.locator(`${paletteDialog} input`).fill('> settings');
  await expect(page.getByRole('option', { name: /Settings/ })).toBeVisible();
  await page.keyboard.press('Enter');

  await expect(page).toHaveURL(/\/settings/);
  expect(errors).toHaveLength(0);
});

test('search palette: "> " lists commands (Reload, Toggle Theme, Settings)', async ({ page }) => {
  await page.goto('/');
  await openPalette(page);

  await page.locator(`${paletteDialog} input`).fill('> ');
  await expect(page.getByRole('option', { name: /Reload Resources/ })).toBeVisible();
  await expect(page.getByRole('option', { name: /Toggle Theme/ })).toBeVisible();
  await expect(page.getByRole('option', { name: /Settings/ })).toBeVisible();
});

test('search palette: Escape closes the palette', async ({ page }) => {
  await page.goto('/');
  await openPalette(page);

  await page.keyboard.press('Escape');
  await expect(page.locator(paletteDialog)).not.toBeVisible();
});

test('search palette: selecting a command does not cause infinite recursion', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (err) => errors.push(err.message));

  await page.goto('/');
  await openPalette(page);

  await page.locator(`${paletteDialog} input`).fill('> ');
  await expect(page.getByRole('option', { name: /Reload Resources/ })).toBeVisible();
  await page.getByRole('option', { name: /Reload Resources/ }).click();

  await expect(page.locator(paletteDialog)).not.toBeVisible();
  expect(errors).toHaveLength(0);
});
