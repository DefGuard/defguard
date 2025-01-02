import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { NetworkForm, User } from '../../types';
import { apiCreateUser } from '../../utils/api/users';
import { loginBasic } from '../../utils/controllers/login';
import { createNetwork } from '../../utils/controllers/vpn/createNetwork';
import { createNetworkDevice } from '../../utils/controllers/vpn/createNetworkDevice';
import { dockerDown, dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';

const testKeys = {
  private: '4K1BwtDCd0XUwq6WThkrQ4/DQ4vIpyEki5aIokqx21c=',
  public: 'n4/imOLXU35yYkWNHvZk2RZfI3l1NwVd4bJsuhTuRzw=',
};

const testNetwork: NetworkForm = {
  name: 'test devices',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  port: '5055',
};

test.describe('Network devices', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    const context = await browser.newContext();
    const page = await context.newPage();
    // wait for fronted
    await waitForBase(page);
    // prepare test network
    await createNetwork(browser, testNetwork);
    // make test user
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUser(page, testUser);
    await context.close();
  });

  test.afterAll(() => dockerDown());

  test('Network devices CRUD', async ({ page, browser }) => {
    const deviceName = 'test';
    const deviceDesc = 'test device description';
    await waitForBase(page);
    await createNetworkDevice(browser, defaultUserAdmin, {
      name: deviceName,
      pubKey: testKeys.public,
      description: deviceDesc,
    });
    await loginBasic(page, defaultUserAdmin);

    // Check if the device is really there
    await page.goto(routes.base + routes.admin.devices);
    const deviceList = await page.locator('#devices-page-devices-list').first();
    const deviceRows = deviceList.locator('.device-row');
    await expect(deviceRows).toHaveCount(1);
    const deviceRow = await deviceRows.first();
    const name = await deviceRow.locator('.cell-1').first().innerText();
    expect(name).toBe(deviceName);
    const location = await deviceRow.locator('.cell-2').first().innerText();
    expect(location).toBe(testNetwork.name);
    const desc = await deviceRow.locator('.cell-4').first().innerText();
    expect(desc).toBe(deviceDesc);

    // Try to edit the device
    await deviceRow.locator('.edit-button').click();
    const editMenu = await page.locator('.edit-button-floating-ui').first();
    await editMenu.getByRole('button', { name: 'Edit' }).click();
    const configCard = page.locator('#edit-standalone-device-modal');
    const deviceNameInput = await configCard.getByTestId('field-name');
    await deviceNameInput.fill(deviceName + '-test');
    const responsePromise = page.waitForResponse('**/device/network');
    await configCard.locator('button').filter({ hasText: 'Submit' }).first().click();
    const response = await responsePromise;
    expect(response.status()).toBe(200);
    const newName = await deviceRow.locator('.cell-1').first().innerText();
    expect(newName).toBe(deviceName + '-test');
  });
});
