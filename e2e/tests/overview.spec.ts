import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { NetworkForm } from '../types';
import { loginBasic } from '../utils/controllers/login';
import { adoptGateway } from '../utils/controllers/vpn/adoptGateway';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart, dockerRestartGateway } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const testNetwork: NetworkForm = {
  name: 'overview location',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  port: '5055',
};

test.describe('VPN overview', () => {
  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    await createRegularLocation(browser, testNetwork);
  });

  test('Change stats period', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.vpnOverview);

    const periodSelect = page.locator('#locations-overview-page .select');
    await expect(periodSelect).toContainText('1h period');
    await periodSelect.getByText('1h period').click();
    await page
      .locator('.select-floating .select-option', { hasText: '8h period' })
      .click();
    await expect(page).toHaveURL(/period=8/);
    await expect(periodSelect).toContainText('8h period');

    await page.reload();
    await expect(periodSelect).toContainText('8h period');
  });

  test('Location details show empty states', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.vpnOverview);

    await expect(page.getByText('All locations summary')).toBeVisible();
    await expect(page.getByText('No data.')).toBeVisible();
    const locationCard = page.locator('li', { hasText: testNetwork.name });
    await expect(locationCard).toContainText('None connected');
    await locationCard.getByRole('button', { name: 'Details', exact: true }).click();

    const detailsPage = page.locator('#location-overview-page');
    await expect(page).toHaveURL(/\/vpn-overview\/\d+/);
    await expect(detailsPage).toContainText("Connected users' devices");
    await expect(detailsPage).toContainText('No connected users');
    await detailsPage.locator('.tab', { hasText: 'Network devices' }).click();
    await expect(detailsPage).toContainText('Connected network devices');
    await expect(detailsPage).toContainText('No connected network devices');
  });

  test('Gateway status of a location', async ({ page, browser }) => {
    const gatewayName = 'overview-gateway';
    dockerRestartGateway();
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.vpnOverview);

    const locationCard = page.locator('li', { hasText: testNetwork.name });
    await expect(locationCard).toContainText('None connected');

    await adoptGateway(browser, testNetwork.name, gatewayName);
    await page.reload();
    await expect(locationCard).toContainText('All gateways connected');

    await locationCard.locator('.gateways-status-badge').click();
    const popover = page.locator('.gateways-status-floating');
    await expect(popover).toContainText('Connected');
    await expect(popover).toContainText(gatewayName);
    await expect(popover).toContainText('Add more gateways');
  });
});
