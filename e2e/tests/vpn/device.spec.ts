import { expect, test } from '@playwright/test';

import { NetworkForm } from '../../types';
import { apiGetUserProfile } from '../../utils/api/users';
import { createUser } from '../../utils/controllers/createUser';
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
  test.beforeAll(() => dockerRestart());
  test.afterAll(() => dockerDown());
  test.afterEach(() => {
    dockerRestart();
  });

  test.beforeEach(async ({ browser }) => {
    const context = await browser.newContext();
    const page = await context.newPage();
    await waitForBase(page);
    await createNetwork(context, testNetwork);
  });

  test('Add test user device with generate', async ({ page, context }) => {
    await waitForBase(page);
    const testUser = await createUser(context, 'testgen');
    await createDevice(context, testUser, {
      name: 'test',
    });
    await loginBasic(page, testUser);
    const testUserProfile = await apiGetUserProfile(page, testUser.username);
    expect(testUserProfile.devices.length).toBe(1);
    const createdDevice = testUserProfile.devices[0];
    expect(createdDevice.networks[0].device_wireguard_ip).toBe('10.10.10.2');
  });

  test('Add test user device with manual', async ({ page, context }) => {
    await waitForBase(page);
    const testUser = await createUser(context, 'testmanual');
    await createDevice(context, testUser, {
      name: 'test',
      pubKey: testKeys.public,
    });
    await loginBasic(page, testUser);
    const testUserProfile = await apiGetUserProfile(page, testUser.username);
    expect(testUserProfile.devices.length).toBe(1);
    const createdDevice = testUserProfile.devices[0];
    expect(createdDevice.networks[0].device_wireguard_ip).toBe('10.10.10.2');
  });
});
