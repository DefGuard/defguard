import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { NetworkForm, User } from '../types';
import { apiCreateUser, apiGetUserDevices } from '../utils/api/users';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic } from '../utils/controllers/login';
import { createDevice } from '../utils/controllers/vpn/createDevice';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../utils/docker';
import { getPageClipboard } from '../utils/getPageClipboard';
import { waitForBase } from '../utils/waitForBase';

const testKeys = {
  private: '4K1BwtDCd0XUwq6WThkrQ4/DQ4vIpyEki5aIokqx21c=',
  public: 'n4/imOLXU35yYkWNHvZk2RZfI3l1NwVd4bJsuhTuRzw=',
};

const testNetwork: NetworkForm = {
  name: 'test devices',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  allowed_ips: ['1.4.2.5'],
  port: '5055',
};

test.describe('Add user device', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };
  const device_name = 'test';
  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    const context = await browser.newContext();
    const page = await context.newPage();
    // wait for fronted
    await waitForBase(page);
    // prepare test network
    await createRegularLocation(browser, testNetwork);
    // make test user
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUser(page, testUser);
    await context.close();
  });

  test('Add test user device with automatically generated public key', async ({
    page,
    browser,
  }) => {
    await waitForBase(page);
    await createDevice(browser, testUser, {
      name: device_name,
    });
    await loginBasic(page, testUser);
    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.devices,
    );
    const devices = await apiGetUserDevices(page, testUser.username);
    expect(devices.length).toBe(1);
    expect(devices[0].name).toBe(device_name);
  });

  test('Add test user device with provided public key', async ({ page, browser }) => {
    await waitForBase(page);
    await createDevice(browser, testUser, {
      name: 'test',
      pubKey: testKeys.public,
    });
    await loginBasic(page, testUser);
    const devices = await apiGetUserDevices(page, testUser.username);
    expect(devices.length).toBe(1);
    expect(devices[0].name).toBe(device_name);
    expect(devices[0].wireguard_pubkey).toBe(testKeys.public);
  });

  test('List user devices', async ({ page, browser }) => {
    const names = ['first device', 'second device'];
    await waitForBase(page);
    for (const name of names) {
      await createDevice(browser, testUser, { name });
    }
    await loginBasic(page, defaultUserAdmin);
    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.devices,
    );

    const rows = page.locator('#profile-devices-table .virtual-row');
    await expect(rows).toHaveCount(names.length);
    for (const name of names) {
      await expect(rows.filter({ hasText: name })).toBeVisible();
    }
    await expect(page.getByTestId('add-device')).toBeEnabled();
  });

  test('Edit user device', async ({ page, browser }) => {
    const newName = 'edited device';
    await waitForBase(page);
    await createDevice(browser, testUser, { name: device_name });
    await loginBasic(page, defaultUserAdmin);
    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.devices,
    );

    const row = page.locator('.virtual-row').filter({ hasText: device_name });
    await row.locator('.edit-cell .icon-button').click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    const modal = page.locator('.modal').filter({ hasText: 'Edit device' });
    await modal.waitFor({ state: 'visible' });
    await modal.getByTestId('field-name').fill(newName);
    await modal.getByRole('button', { name: 'Save changes', exact: true }).click();
    await modal.waitFor({ state: 'hidden' });

    await page.reload();
    const rows = page.locator('#profile-devices-table .virtual-row');
    await expect(rows).toHaveCount(1);
    await expect(rows.filter({ hasText: newName })).toBeVisible();
  });

  test('Delete user device', async ({ page, browser }) => {
    await waitForBase(page);
    await createDevice(browser, testUser, { name: device_name });
    await loginBasic(page, defaultUserAdmin);
    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.devices,
    );

    const row = page.locator('.virtual-row').filter({ hasText: device_name });
    await row.locator('.edit-cell .icon-button').click();
    await page.locator('.menu').getByText('Delete', { exact: true }).click();
    await page.getByTestId('confirm-action-submit').click();

    await expect(page.getByText('No devices', { exact: true })).toBeVisible();
    await page.reload();
    await expect(page.getByText('No devices', { exact: true })).toBeVisible();
    await expect(page.locator('#profile-devices-table .virtual-row')).toHaveCount(0);
  });

  test('Show user device configuration', async ({ page, browser }) => {
    await waitForBase(page);
    await createDevice(browser, testUser, { name: device_name });
    await loginBasic(page, defaultUserAdmin);
    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.devices,
    );

    const row = page.locator('.virtual-row').filter({ hasText: device_name });
    await row.locator('.edit-cell .icon-button').click();
    await page.locator('.menu').getByText('Show configuration', { exact: true }).click();
    const modal = page.locator('#user-device-config-modal');
    await modal.waitFor({ state: 'visible' });
    await expect(modal).toContainText(testNetwork.name);

    await modal.getByRole('button', { name: 'Copy to clipboard', exact: true }).click();
    const config = await getPageClipboard(page);
    expect(config).toContain('[Interface]');
    expect(config).toContain('[Peer]');
    expect(config).toContain(`${testNetwork.endpoint}:${testNetwork.port}`);

    await modal.getByRole('button', { name: 'Close', exact: true }).click();
    await modal.waitFor({ state: 'hidden' });
  });
});

test.describe('Add user device without locations', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(() => dockerRestart());

  test('Add new device is disabled without locations', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.devices,
    );
    await expect(page.getByText('No devices', { exact: true })).toBeVisible();
    await expect(page.getByTestId('add-device')).toBeDisabled();
  });
});
