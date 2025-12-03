import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { NetworkForm, User } from '../../types';
import { apiCreateUser } from '../../utils/api/users';
import { loginBasic } from '../../utils/controllers/login';
import { createRegularLocation } from '../../utils/controllers/vpn/createNetwork';
import {
  createNetworkCLIDevice,
  doAction,
  editNetworkDevice,
  getDeviceRow,
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
    const urlMatch = command?.match(/-u\s+(\S+)/);
    const tokenMatch = command?.match(/-t\s+(\S+)/);

    const url = urlMatch?.[1];
    const token = tokenMatch?.[1];
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
    await waitForPromise(2000);
    await page.goto(routes.base + routes.network_devices);
    await waitForPromise(2000);
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
