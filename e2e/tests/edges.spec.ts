import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const EDGE_NAME = 'edge-test';

test.describe('Edge components', () => {
  test.beforeEach(() => dockerRestart());

  test('List edge components', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.edges);

    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(1);
    await expect(rows.first()).toContainText(EDGE_NAME);
    await expect(rows.first()).toContainText('50051');
    await expect(rows.first()).toContainText('Connected');
    await expect(page.getByTestId('add-edge')).toBeVisible();
  });

  test('Edit edge component name', async ({ page }) => {
    const newName = 'renamed-edge';
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.edges);

    const row = page.locator('.virtual-row').filter({ hasText: EDGE_NAME });
    await row.locator('.edit-cell .icon-button').click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    await page.waitForURL('**/edit');
    await page.getByTestId('field-name').fill(newName);
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page.getByText('Edge Component updated')).toBeVisible();

    await page.goto(routes.base + routes.edges);
    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(1);
    await expect(rows.first()).toContainText(newName);
  });

  test('Disable and enable edge component', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.edges);

    const row = page.locator('.virtual-row').filter({ hasText: EDGE_NAME });
    await row.locator('.edit-cell .icon-button').click();
    await page.locator('.menu').getByText('Disable', { exact: true }).click();
    await expect(row).toContainText('Disabled');
    await page.reload();
    await expect(row).toContainText('Disabled');

    await row.locator('.edit-cell .icon-button').click();
    await page.locator('.menu').getByText('Enable', { exact: true }).click();
    await expect(row).not.toContainText('Disabled');
    await expect(async () => {
      await page.reload();
      await expect(row).toContainText('Connected', { timeout: 1_000 });
    }).toPass({ timeout: 30_000 });
  });
});
