import { expect, test } from '@playwright/test';

import { defaultUserAdmin, testUserTemplate } from '../../config';
import { NetworkForm, User } from '../../types';
import { apiCreateUser, apiGetUserProfile } from '../../utils/api/users';
import { loginBasic } from '../../utils/controllers/login';
import { createDevice } from '../../utils/controllers/vpn/createDevice';
import { createNetwork } from '../../utils/controllers/vpn/createNetwork';
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

test.describe('Add user device', () => {
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

  test('Add test user device with generate', async ({ page, browser }) => {
    await waitForBase(page);
    await createDevice(browser, testUser, {
      name: 'test',
    });
    await loginBasic(page, testUser);
    const testUserProfile = await apiGetUserProfile(page, testUser.username);
    expect(testUserProfile.devices.length).toBe(1);
    const createdDevice = testUserProfile.devices[0];
    expect(createdDevice.networks[0].device_wireguard_ips).toStrictEqual(['10.10.10.2']);
  });

  test('Add test user device with manual', async ({ page, browser }) => {
    await waitForBase(page);
    await createDevice(browser, testUser, {
      name: 'test',
      pubKey: testKeys.public,
    });
    await loginBasic(page, testUser);
    const testUserProfile = await apiGetUserProfile(page, testUser.username);
    expect(testUserProfile.devices.length).toBe(1);
    const createdDevice = testUserProfile.devices[0];
    expect(createdDevice.networks[0].device_wireguard_ips).toStrictEqual(['10.10.10.2']);
  });
});
