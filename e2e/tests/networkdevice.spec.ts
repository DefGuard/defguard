import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { NetworkForm, User } from '../types';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic } from '../utils/controllers/login';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import {
  createNetworkCLIDevice,
  startNetworkDeviceEnrollment,
} from '../utils/controllers/vpn/createNetworkDevice';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

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
    const config = configs.pop();
    expect(config['endpoint']).toEqual(`${testNetwork.endpoint}:${testNetwork.port}`);
  });

  test('Create Manual WireGuard Client network device', async ({ page, browser }) => {
    const deviceName = 'test';
    const deviceDesc = 'test device description';
    await waitForBase(page);
    await startNetworkDeviceEnrollment(browser, defaultUserAdmin, {
      name: deviceName,
      pubKey: testKeys.public,
      description: deviceDesc,
    });
    await startNetworkDeviceEnrollment(browser, defaultUserAdmin, {
      name: deviceName + '2',
      description: deviceDesc,
    });
  });

  test('List network devices', async ({ page, browser }) => {
    const devices = [
      { name: 'first-device', description: 'first description' },
      { name: 'second-device', description: 'second description' },
    ];
    await waitForBase(page);
    for (const device of devices) {
      await createNetworkCLIDevice(browser, defaultUserAdmin, device);
    }
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.network_devices);

    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(devices.length);
    for (const device of devices) {
      const row = rows.filter({ hasText: device.name });
      await expect(row).toContainText(device.description);
      await expect(row).toContainText(testNetwork.name);
      await expect(row).toContainText(defaultUserAdmin.username);
      await expect(row).toContainText('Awaiting Setup');
    }
  });

  test('Edit network device', async ({ page, browser }) => {
    const device = { name: 'edit-device', description: 'before edit' };
    const updated = { name: 'edited-device', description: 'after edit' };
    await waitForBase(page);
    await createNetworkCLIDevice(browser, defaultUserAdmin, device);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.network_devices);

    const row = page.locator('.virtual-row').filter({ hasText: device.name });
    await row.locator('.icon-button').click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    const modal = page.locator('#edit-network-device-modal');
    await modal.waitFor({ state: 'visible' });
    await modal.getByTestId('field-name').fill(updated.name);
    await modal.getByTestId('field-description').fill(updated.description);
    const responsePromise = page.waitForResponse(
      (resp) =>
        resp.url().includes('/device/network/') && resp.request().method() === 'PUT',
    );
    await modal.getByRole('button', { name: 'Save changes', exact: true }).click();
    expect((await responsePromise).status()).toBe(200);
    await modal.waitFor({ state: 'hidden' });

    await page.reload();
    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(1);
    await expect(rows.filter({ hasText: updated.name })).toContainText(
      updated.description,
    );
    await expect(rows.filter({ hasText: device.name })).toHaveCount(0);
  });

  test('Delete network device', async ({ page, browser }) => {
    const device = { name: 'delete-device', description: 'to be deleted' };
    await waitForBase(page);
    await createNetworkCLIDevice(browser, defaultUserAdmin, device);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.network_devices);

    const row = page.locator('.virtual-row').filter({ hasText: device.name });
    await row.locator('.icon-button').click();
    await page.locator('.menu').getByText('Delete', { exact: true }).click();
    await page.locator('button[data-variant="critical"]').click();

    await expect(page.getByText("You don't have any network devices.")).toBeVisible();
    await page.reload();
    await expect(page.getByText("You don't have any network devices.")).toBeVisible();
    await expect(page.locator('.virtual-row')).toHaveCount(0);
  });
});
