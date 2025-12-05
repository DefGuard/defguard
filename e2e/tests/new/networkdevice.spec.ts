import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { NetworkForm, User } from '../../types';
import { apiCreateUser } from '../../utils/api/users';
import { loginBasic } from '../../utils/controllers/login';
import { createRegularLocation } from '../../utils/controllers/vpn/createNetwork';
import {
  createNetworkCLIDevice,
  startNetworkDeviceEnrollment,
} from '../../utils/controllers/vpn/createNetworkDevice';
import { dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';
import { waitForRoute } from '../../utils/waitForRoute';
import { createUser } from '../../utils/controllers/createUser';
import { waitForPromise } from '../../utils/waitForPromise';

const testKeys = {
  private: '4K1BwtDCd0XUwq6WThkrQ4/DQ4vIpyEki5aIokqx21c=',
  public: 'n4/imOLXU35yYkWNHvZk2RZfI3l1NwVd4bJsuhTuRzw=',
};

const testNetwork: NetworkForm = {
  name: 'test devices',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  allowed_ips: ['1.2.4.5'],
  port: '5055',
};

test.describe('Network devices', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    const context = await browser.newContext();
    const page = await context.newPage();
    await waitForBase(page);
    await createRegularLocation(browser, testNetwork);
    await loginBasic(page, defaultUserAdmin);
    await createUser(browser, testUser);
    await context.close();
  });

  //  TODO:Check this
  // // View the config
  // await doAction({ page, deviceRow, action: 'View config' });
  // const configDisplayCard = page.locator('#standalone-device-config-modal');
  // const config = await configDisplayCard.locator('.config').first().innerText();
  // expect(config).toContain(`${testNetwork.endpoint}:${testNetwork.port}`);
  // await configDisplayCard.getByRole('button', { name: 'Close' }).click();

  // // Generate the token command
  // await doAction({ page, deviceRow, action: 'Generate auth token' });
  // const tokenCard = page.locator('.modal-content');
  // const command = await tokenCard.locator('.expanded-content').first().innerText();
  // expect(command.length).toBeGreaterThan(0);
  // await tokenCard.getByRole('button', { name: 'Close' }).click();

  // // Delete device
  // await doAction({ page, deviceRow, action: 'Delete' });
  // const deleteModal = page.locator('.modal');
  // await deleteModal.getByRole('button', { name: 'Delete' }).click();
  // await expect(deviceRows).toHaveCount(0);

  test('Create and setup Defguard CLI network device', async ({
    page,
    browser,
    request,
  }) => {
    const deviceName = 'test';
    const deviceDesc = 'test device description';
    await waitForBase(page);
    await createNetworkCLIDevice(browser, defaultUserAdmin, {
      name: deviceName,
      pubKey: testKeys.public,
      description: deviceDesc,
    });
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.network_devices);
    const deviceRow = page.locator('.virtual-row').filter({ hasText: deviceName });
    await expect(deviceRow).toContainText('Awaiting Setup');
    await expect(deviceRow).toContainText(deviceName);
    await expect(deviceRow).toContainText(testNetwork.name);
    await expect(deviceRow).toContainText(defaultUserAdmin.username);
    await deviceRow.locator('.icon-button').click();
    await page.getByTestId('generate-auth-token').click();
    const command = await page.getByTestId('copy-field').locator('p').textContent();
    await page.getByTestId('close').click();
    const tokenMatch = command?.match(/-t\s+(\S+)/);

    const token = tokenMatch?.[1];
    const res = await request.post(`http://localhost:8080/api/v1/enrollment/start`, {
      data: {
        token,
      },
    });
    expect(res.status()).toBe(200);
    const responsePayload = await res.json();
    expect(responsePayload).toHaveProperty('instance');
    const createDeviceRes = await request.post(
      `http://localhost:8080/api/v1/enrollment/create_device`,
      {
        data: {
          name: 'dev',
          pubkey: 'DwcCqbwTEvI4erU8RrTUg3fRILhBVzy3rrTqEPGYKIA=',
          token: null,
        },
      },
    );
    expect(createDeviceRes.status()).toBe(200);
    const createDeviceResPayload = await createDeviceRes.json();
    expect(createDeviceResPayload).toHaveProperty('configs');
    const configs = createDeviceResPayload['configs'];
    expect(configs.length).toEqual(1);
    // const config = configs.pop();
    // expect(config['endpoint']).toEqual(`${testNetwork.endpoint}:${testNetwork.port}`); // FIXME: add this after wizard is fixed
  });

  test('Create Manual WireGuard Client network device', async ({ page, browser }) => {
    const deviceName = 'test';
    const deviceDesc = 'test device description';
    await waitForBase(page);
    // await startNetworkDeviceEnrollment(browser, defaultUserAdmin, {
    //   name: deviceName,
    //   pubKey: testKeys.public,
    //   description: deviceDesc, // FIXME: Currently broken by frontend.
    // });
    await startNetworkDeviceEnrollment(browser, defaultUserAdmin, {
      name: deviceName + '2',
      description: deviceDesc,
    });
  });
});
