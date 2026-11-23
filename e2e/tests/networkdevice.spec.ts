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

const testPresharedKey = 'oZFW7IISUedr8RLjI1On2ZoEg4RkCppwGu4BILPwj9Q=';

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

  test('Manual WireGuard Client network device with preshared key', async ({
    page,
    browser,
  }) => {
    await waitForBase(page);
    const providedConfig = await startNetworkDeviceEnrollment(browser, defaultUserAdmin, {
      name: 'psk-provided',
      pubKey: testKeys.public,
      presharedKey: testPresharedKey,
    });
    expect(providedConfig).toContain(`PresharedKey = ${testPresharedKey}`);
    const generatedConfig = await startNetworkDeviceEnrollment(
      browser,
      defaultUserAdmin,
      {
        name: 'psk-generated',
        generatePresharedKey: true,
      },
    );
    expect(generatedConfig).toMatch(/PresharedKey = [A-Za-z0-9+/]{43}=/);

    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.network_devices);
    const deviceRow = page.locator('.virtual-row').filter({ hasText: 'psk-provided' });
    const modal = page.locator('#edit-network-device-modal');
    const openEditModal = async () => {
      await deviceRow.locator('.icon-button').click();
      await page.getByTestId('edit-network-device').click();
      await expect(modal.getByTestId('field-preshared_key')).toBeVisible();
    };
    const saveChanges = async () => {
      const responsePromise = page.waitForResponse(
        (response) =>
          response.url().includes('/api/v1/device/network/') &&
          response.request().method() === 'PUT',
      );
      await modal.getByRole('button', { name: 'Save changes' }).click();
      const response = await responsePromise;
      expect(response.status()).toBe(200);
      return {
        request: response.request().postDataJSON(),
        device: await response.json(),
      };
    };

    await openEditModal();
    const untouched = await saveChanges();
    expect(untouched.request).not.toHaveProperty('preshared_key');
    expect(untouched.device.preshared_key_set).toBe(true);
    await expect(modal).toBeHidden();

    const replacementKey = 'k6V0JY0n8jXkhs1xMkRqTqJbE1hYyRnQbRUx5vtcO2s=';
    await openEditModal();
    await modal.getByTestId('field-preshared_key').fill(`  ${replacementKey}  `);
    const replaced = await saveChanges();
    expect(replaced.request.preshared_key).toBe(replacementKey);
    expect(replaced.device.preshared_key_set).toBe(true);
    await expect(modal).toBeHidden();

    await openEditModal();
    await modal.getByTestId('field-generatePresharedKey').click();
    const generated = await saveChanges();
    expect(generated.request.preshared_key).toMatch(/^[A-Za-z0-9+/]{43}=$/);
    expect(generated.request.preshared_key).not.toBe(replacementKey);
    await expect(modal.locator('.copy-field')).toContainText(
      generated.request.preshared_key,
    );
    await modal.getByTestId('finish').click();
    await expect(modal).toBeHidden();

    await openEditModal();
    await modal.getByTestId('field-usePresharedKey').locator('.interactive').click();
    const removed = await saveChanges();
    expect(removed.request.preshared_key).toBeNull();
    expect(removed.device.preshared_key_set).toBe(false);
  });
});
