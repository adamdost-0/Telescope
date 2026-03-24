import { expect, test } from '@playwright/test';
import type { Page } from '@playwright/test';
import { installMockTauri } from './helpers/mock-tauri';

const paletteDialog = '[role="dialog"]';

type RouteCase = {
  title: string;
  path: string;
  rowName: string;
  columns: string[];
  detailPathPattern: RegExp;
  paletteQuery: string;
  emptyMessage: string;
};

const routeCases: RouteCase[] = [
  {
    title: 'ReplicaSets',
    path: '/resources/replicasets',
    rowName: 'nginx-deploy-7f8f9c5c6f',
    columns: ['Name', 'Namespace', 'Ready', 'Age'],
    detailPathPattern: /\/resources\/replicasets\/default\/nginx-deploy-7f8f9c5c6f/,
    paletteQuery: '@ nginx-deploy-7f8f9c5c6f',
    emptyMessage: 'No replica sets found in this namespace.',
  },
  {
    title: 'ClusterRoles',
    path: '/resources/clusterroles',
    rowName: 'view',
    columns: ['Name', 'Created', 'Rules'],
    detailPathPattern: /\/resources\/clusterroles\/_cluster\/view/,
    paletteQuery: '@ view',
    emptyMessage: 'No cluster roles found.',
  },
  {
    title: 'ClusterRoleBindings',
    path: '/resources/clusterrolebindings',
    rowName: 'viewers-binding',
    columns: ['Name', 'Role Ref', 'Subjects', 'Created'],
    detailPathPattern: /\/resources\/clusterrolebindings\/_cluster\/viewers-binding/,
    paletteQuery: '@ viewers-binding',
    emptyMessage: 'No cluster role bindings found.',
  },
];

async function openPalette(page: Page) {
  await expect(page.locator('.app-shell')).toBeVisible({ timeout: 15000 });
  await page.evaluate(() => {
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true }));
  });
  await expect(page.locator(paletteDialog)).toBeVisible();
}

for (const routeCase of routeCases) {
  test(`${routeCase.title}: route loads table, columns render, and detail click-through works`, async ({ page }) => {
    await installMockTauri(page);
    await page.goto(routeCase.path);
    await expect(page.locator('.app-shell')).toBeVisible({ timeout: 15000 });

    await expect(page.getByRole('heading', { name: routeCase.title })).toBeVisible();
    const table = page.getByRole('table', { name: 'Resource list' });
    await expect(table).toBeVisible();

    for (const column of routeCase.columns) {
      await expect(
        table.getByRole('columnheader', { name: new RegExp(`^Sort by ${column}$`, 'i') })
      ).toBeVisible();
    }

    await table.getByRole('link', { name: routeCase.rowName, exact: true }).click();
    await expect(page).toHaveURL(routeCase.detailPathPattern);
  });

  test(`${routeCase.title}: search palette finds resources and navigates to detail`, async ({ page }) => {
    await installMockTauri(page);
    await page.goto('/overview');

    await openPalette(page);
    await page.locator(`${paletteDialog} input`).fill(routeCase.paletteQuery);
    const option = page.locator(`${paletteDialog} [role="option"]`, { hasText: routeCase.rowName }).first();
    await expect(option).toBeVisible();
    await option.click();

    await expect(page).toHaveURL(routeCase.detailPathPattern);
  });

  test(`${routeCase.title}: loading state renders then table appears`, async ({ page }) => {
    await installMockTauri(page, {
      commandDelays: {
        get_resources: 1000,
      },
    });
    await page.goto(routeCase.path);
    await expect(page.getByRole('status', { name: 'Loading data' })).toBeVisible();
    await expect(page.getByRole('status', { name: 'Loading data' })).toHaveCount(0);
    await expect(page.getByRole('table', { name: 'Resource list' })).toBeVisible();
  });

  test(`${routeCase.title}: command errors fall back to empty state`, async ({ page }) => {
    await installMockTauri(page, {
      commandErrors: {
        get_resources: `Failed to load ${routeCase.title.toLowerCase()}`,
      },
    });
    await page.goto(routeCase.path);
    await expect(page.getByRole('table', { name: 'Resource list' })).toHaveCount(0);
    await expect(page.getByRole('alert')).toHaveCount(0);
    await expect(page.getByText(routeCase.emptyMessage)).toBeVisible();
  });
}
