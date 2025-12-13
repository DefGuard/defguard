import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { NetworkForm, User } from '../../types';
import { apiCreateUser, apiGetUserProfile } from '../../utils/api/users';
import { loginBasic } from '../../utils/controllers/login';
import { createDevice } from '../../utils/controllers/vpn/createDevice';
import { createRegularLocation } from '../../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';

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
    const testUserProfile = await apiGetUserProfile(page, testUser.username);
    expect(testUserProfile.devices.length).toBe(1);
    expect(testUserProfile.devices[0].name).toBe(device_name);
  });

  test('Add test user device with provided public key', async ({ page, browser }) => {
    await waitForBase(page);
    await createDevice(browser, testUser, {
      name: 'test',
      pubKey: testKeys.public,
    });
    await loginBasic(page, testUser);
    const testUserProfile = await apiGetUserProfile(page, testUser.username);
    expect(testUserProfile.devices.length).toBe(1);
    expect(testUserProfile.devices[0].name).toBe(device_name);
    expect(testUserProfile.devices[0].wireguard_pubkey).toBe(testKeys.public);
  });
});
