import { expect, test } from '@playwright/test';
import { installMockTauri } from './helpers/mock-tauri';

const releaseName = 'ingress-nginx';
const releaseNamespace = 'default';

async function openUninstallDialog(page: import('@playwright/test').Page) {
  await page.goto('/helm');
  await expect(page.locator('.app-shell')).toBeVisible({ timeout: 15000 });
  // Use data-testid to avoid coupling to iconography/emoji in headings
  await expect(page.getByTestId('helm-releases-heading')).toBeVisible();

  const row = page.locator('tbody tr').filter({ hasText: releaseName }).first();
  await expect(row).toBeVisible();

  const uninstallButton = row.getByRole('button', { name: /uninstall/i });
  await expect(uninstallButton).toBeVisible();
  await uninstallButton.click();

  const dialog = page.getByRole('dialog').filter({ hasText: new RegExp(releaseName, 'i') });
  await expect(dialog).toBeVisible();
  return dialog;
}

test('helm uninstall: list shows uninstall action, confirms, triggers IPC, and shows success', async ({ page }) => {
  await installMockTauri(page);

  const dialog = await openUninstallDialog(page);
  await dialog.getByRole('button', { name: /confirm|uninstall/i }).click();

  await expect.poll(async () => page.evaluate(() => {
    const tauri = (window as Window & { __TEST_TAURI__: { calls: Array<{ cmd: string; args: { namespace?: string; name?: string } }> } }).__TEST_TAURI__;
    return tauri.calls.find((entry) => entry.cmd === 'helm_uninstall') ?? null;
  })).toMatchObject({
    cmd: 'helm_uninstall',
    args: { namespace: releaseNamespace, name: releaseName },
  });

  await expect(page.getByText(/uninstalled helm release/i)).toBeVisible();
});

test('helm uninstall: command error surfaces an error message', async ({ page }) => {
  await installMockTauri(page, {
    commandErrors: {
      helm_uninstall: 'Helm uninstall failed: release not found.',
    },
  });

  const dialog = await openUninstallDialog(page);
  await dialog.getByRole('button', { name: /confirm|uninstall/i }).click();

  await expect.poll(async () => page.evaluate(() => {
    const tauri = (window as Window & { __TEST_TAURI__: { calls: Array<{ cmd: string }> } }).__TEST_TAURI__;
    return tauri.calls.some((entry) => entry.cmd === 'helm_uninstall');
  })).toBe(true);

  await expect(page.getByText(/helm uninstall failed|release not found/i)).toBeVisible();
});
