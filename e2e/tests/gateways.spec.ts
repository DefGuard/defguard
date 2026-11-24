import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { NetworkForm } from '../types';
import { loginBasic } from '../utils/controllers/login';
import { adoptGateway } from '../utils/controllers/vpn/adoptGateway';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart, dockerRestartGateway } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const GATEWAY_NAME = 'test-gateway';

const testNetwork: NetworkForm = {
  name: 'gateway location',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  port: '5055',
};

test.describe('Gateways', () => {
  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    dockerRestartGateway();
    await createRegularLocation(browser, testNetwork);
    await adoptGateway(browser, testNetwork.name, GATEWAY_NAME);
  });

  test('List gateways', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.locations);
    await page.locator('.tab', { hasText: 'Gateways' }).click();

    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(1);
    await expect(rows.first()).toContainText(GATEWAY_NAME);
    await expect(rows.first()).toContainText(testNetwork.name);
    await expect(rows.first()).toContainText('gateway');
    await expect(rows.first()).toContainText('50066');
    await expect(rows.first()).toContainText('Connected');
  });

  test('Edit gateway name', async ({ page }) => {
    const newName = 'renamed-gateway';
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.locations);
    await page.locator('.tab', { hasText: 'Gateways' }).click();

    const row = page.locator('.virtual-row').filter({ hasText: GATEWAY_NAME });
    await row.locator('.edit-cell .icon-button').click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    await page.waitForURL('**/edit');
    await page.getByTestId('field-name').fill(newName);
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page.getByText('Gateway updated')).toBeVisible();

    await page.goto(routes.base + routes.locations);
    await page.locator('.tab', { hasText: 'Gateways' }).click();
    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(1);
    await expect(rows.first()).toContainText(newName);
    await expect(rows.first()).toContainText(testNetwork.name);
  });

  test('Disable and enable gateway', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.locations);
    const gatewaysTab = page.locator('.tab', { hasText: 'Gateways' });
    await gatewaysTab.click();

    const row = page.locator('.virtual-row').filter({ hasText: GATEWAY_NAME });
    await row.locator('.edit-cell .icon-button').click();
    await page.locator('.menu').getByText('Disable', { exact: true }).click();
    await expect(row).toContainText('Disabled');
    await page.reload();
    await gatewaysTab.click();
    await expect(row).toContainText('Disabled');

    await row.locator('.edit-cell .icon-button').click();
    await page.locator('.menu').getByText('Enable', { exact: true }).click();
    await expect(row).not.toContainText('Disabled');
    await expect(async () => {
      await page.reload();
      await gatewaysTab.click();
      await expect(row).toContainText('Connected', { timeout: 1_000 });
    }).toPass({ timeout: 30_000 });
  });

  test('Delete gateway', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.locations);
    const gatewaysTab = page.locator('.tab', { hasText: 'Gateways' });
    await gatewaysTab.click();

    const row = page.locator('.virtual-row').filter({ hasText: GATEWAY_NAME });
    await row.locator('.edit-cell .icon-button').click();
    await page.locator('.menu').getByText('Delete', { exact: true }).click();
    const modal = page.locator('#action-modal');
    await expect(modal).toContainText(GATEWAY_NAME);
    await expect(modal).toContainText(testNetwork.name);
    await page.getByTestId('confirm-action-submit').click();

    await expect(page.getByText('No gateways found')).toBeVisible();
    await page.reload();
    await gatewaysTab.click();
    await expect(page.getByText('No gateways found')).toBeVisible();
  });
});
