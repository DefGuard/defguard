import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { NetworkForm, User } from '../types';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic } from '../utils/controllers/login';
import { createDevice } from '../utils/controllers/vpn/createDevice';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const testNetwork: NetworkForm = {
  name: 'static ip location',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  port: '5055',
};

test.describe('Static IP assignment', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    await createRegularLocation(browser, testNetwork);
    await createUser(browser, testUser);
  });

  test('Assign and modify static IP', async ({ page, browser }) => {
    await waitForBase(page);
    await createDevice(browser, testUser, { name: 'first device' });
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.identity.users);

    const userRow = page.locator('.virtual-row').filter({ hasText: testUser.username });
    await userRow.locator('.edit-cell .icon-button').click();
    await page.getByTestId('assign-ip').click();
    const modal = page.locator('#assign-user-ip-modal');
    await modal.waitFor({ state: 'visible' });
    await modal
      .locator('.location-assignment-card')
      .filter({ hasText: testNetwork.name })
      .locator('.main-track')
      .click();
    const ipInput = modal.locator('.suggested-ip-input input');
    await expect(ipInput).toHaveValue('2');
    const validatePromise = page.waitForResponse('**/ip/validate');
    await ipInput.fill('10');
    expect((await validatePromise).status()).toBe(200);
    await expect(modal.locator('.suggested-ip-input')).not.toContainText('IP address');
    await modal.getByRole('button', { name: 'Submit', exact: true }).click();
    await modal.waitFor({ state: 'hidden' });

    await page.reload();
    await userRow.locator('.edit-cell .icon-button').click();
    await page.getByTestId('assign-ip').click();
    await modal.waitFor({ state: 'visible' });
    await modal
      .locator('.location-assignment-card')
      .filter({ hasText: testNetwork.name })
      .locator('.main-track')
      .click();
    await expect(ipInput).toHaveValue('10');
  });

  test('Static IP validation errors', async ({ page, browser }) => {
    await waitForBase(page);
    await createDevice(browser, testUser, { name: 'first device' });
    await createDevice(browser, testUser, { name: 'second device' });
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.identity.users);

    const userRow = page.locator('.virtual-row').filter({ hasText: testUser.username });
    await userRow.locator('.edit-cell .icon-button').click();
    await page.getByTestId('assign-ip').click();
    const modal = page.locator('#assign-user-ip-modal');
    await modal.waitFor({ state: 'visible' });
    await modal
      .locator('.location-assignment-card')
      .filter({ hasText: testNetwork.name })
      .locator('.main-track')
      .click();

    const firstDevice = modal
      .locator('.ip-assignment-device-section')
      .filter({ hasText: 'first device' });
    const secondDevice = modal
      .locator('.ip-assignment-device-section')
      .filter({ hasText: 'second device' });
    const secondIp = await secondDevice.locator('.suggested-ip-input input').inputValue();
    const firstInput = firstDevice.locator('.suggested-ip-input');

    await firstInput.locator('input').fill(secondIp);
    await expect(firstInput).toContainText(
      `IP address 10.10.10.${secondIp} is already assigned in location ${testNetwork.name}`,
    );

    await firstInput.locator('input').fill('1');
    await expect(firstInput).toContainText(
      `IP address 10.10.10.1 is reserved for Gateway in location ${testNetwork.name}`,
    );

    await firstInput.locator('input').fill('abc');
    await expect(firstInput).toContainText('Invalid or already taken');
  });

  test('No IP settings for user without devices', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.identity.users);

    const userRow = page.locator('.virtual-row').filter({ hasText: testUser.username });
    await userRow.locator('.edit-cell .icon-button').click();
    await expect(page.locator('.menu')).toBeVisible();
    await expect(page.getByTestId('assign-ip')).toHaveCount(0);
  });
});
