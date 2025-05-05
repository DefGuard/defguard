import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { NetworkForm, User } from '../../types';
import { apiCreateUser } from '../../utils/api/users';
import { loginBasic } from '../../utils/controllers/login';
import { createNetwork } from '../../utils/controllers/vpn/createNetwork';
import {
  createNetworkDevice,
  doAction,
  editNetworkDevice,
  getDeviceRow,
  startNetworkDeviceEnrollment,
} from '../../utils/controllers/vpn/createNetworkDevice';
import { dockerDown, dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';
import { waitForRoute } from '../../utils/waitForRoute';

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

  test('Network devices CRUD and actions', async ({ page, browser }) => {
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
    const deviceRow = await getDeviceRow({ page, deviceName });
    const name = await deviceRow.locator('.cell-1').first().innerText();
    expect(name).toBe(deviceName);
    const location = await deviceRow.locator('.cell-2').first().innerText();
    expect(location).toBe(testNetwork.name);
    const desc = await deviceRow.locator('.cell-4').first().innerText();
    expect(desc).toBe(deviceDesc);
    const addedBy = await deviceRow.locator('.cell-5').first().innerText();
    expect(addedBy).toBe(defaultUserAdmin.username);

    // Make sure the device is not displayed on the user's page
    await page.goto(routes.base + routes.me);
    await waitForRoute(page, routes.me);
    const devices = page.locator('.device-card');
    const networkDevice = (await devices.all()).find(async (val) => {
      if ((await val.getByTestId('device-name').innerText()) === deviceName) {
        return true;
      } else {
        return false;
      }
    });
    expect(networkDevice).toBeUndefined();
    await page.goto(routes.base + routes.admin.devices);
    await waitForRoute(page, routes.admin.devices);

    await editNetworkDevice(browser, defaultUserAdmin, deviceName, {
      name: deviceName + '-test',
      description: 'new description',
    });
    await page.reload();
    await waitForRoute(page, routes.admin.devices);
    const newName = await deviceRow.locator('.cell-1').first().innerText();
    expect(newName).toBe(deviceName + '-test');
    const newDesc = await deviceRow.locator('.cell-4').first().innerText();
    expect(newDesc).toBe('new description');

    // View the config
    await doAction({ page, deviceRow, action: 'View config' });
    const configDisplayCard = page.locator('#standalone-device-config-modal');
    const config = await configDisplayCard.locator('.config').first().innerText();
    expect(config).toContain(`${testNetwork.endpoint}:${testNetwork.port}`);
    await configDisplayCard.getByRole('button', { name: 'Close' }).click();

    // Generate the token command
    await doAction({ page, deviceRow, action: 'Generate auth token' });
    const tokenCard = page.locator('.modal-content');
    const command = await tokenCard.locator('.expanded-content').first().innerText();
    expect(command.length).toBeGreaterThan(0);
    await tokenCard.getByRole('button', { name: 'Close' }).click();

    // Delete device
    await doAction({ page, deviceRow, action: 'Delete' });
    const deleteModal = page.locator('.modal');
    await deleteModal.getByRole('button', { name: 'Delete' }).click();
    await expect(deviceRows).toHaveCount(0);
  });

  test('Network devices enrollment', async ({ page, browser, request }) => {
    const deviceName = 'test';
    const deviceDesc = 'test device description';
    await waitForBase(page);
    const command = await startNetworkDeviceEnrollment(browser, defaultUserAdmin, {
      name: deviceName,
      pubKey: testKeys.public,
      description: deviceDesc,
    });
    const urlMatch = command.match(/-u\s+(\S+)/);
    const tokenMatch = command.match(/-t\s+(\S+)/);
    expect(urlMatch).not.toBeNull();
    expect(tokenMatch).not.toBeNull();
    expect(urlMatch?.length).toBeGreaterThan(0);
    expect(tokenMatch?.length).toBeGreaterThan(0);
    const url = urlMatch?.pop() as string;
    const token = tokenMatch?.pop() as string;
    console.log('URL:', url, 'Token:', token);
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
    const config = configs.pop();
    expect(config['endpoint']).toEqual(`${testNetwork.endpoint}:${testNetwork.port}`);
  });
});
