import { expect, test } from '@playwright/test';

import { testsConfig, testUserTemplate } from '../config';
import { NetworkForm, User } from '../types';
import { apiGetUserProfile } from '../utils/api/users';
import {
  createDevice,
  createUserEnrollment,
  password,
  setPassword,
  setToken,
  validateData,
} from '../utils/controllers/enrollment';
import { loginBasic } from '../utils/controllers/login';
import { createNetwork } from '../utils/controllers/vpn/createNetwork';
import { dockerDown, dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';

const testNetwork: NetworkForm = {
  name: 'test network',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  port: '5055',
};

test.describe('Create user with enrollment enabled', () => {
  let token: string;
  const user: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    const response = await createUserEnrollment(browser, user);
    token = response.token;
    await createNetwork(browser, testNetwork);
  });

  test.afterAll(() => {
    dockerDown();
  });

  test('Complete enrollment with created user', async ({ page }) => {
    expect(token).toBeDefined();
    await waitForBase(page);
    await page.goto(testsConfig.ENROLLMENT_URL);
    await waitForPromise(2000);
    await setToken(token, page);
    // Welcome page
    await page.getByTestId('enrollment-next').click();
    // Data validation
    await validateData(user, page);
    await page.getByTestId('enrollment-next').click();
    // Set password
    await setPassword(page);
    // VPN
    await page.getByTestId('enrollment-next').click();
    await createDevice(page);
    // Finish message
    await page.getByTestId('enrollment-next').click({ timeout: 2000 });
    await page.locator('#enrollment-finish-card').waitFor({ state: 'visible' });
    await page.waitForLoadState('networkidle');
    waitForPromise(2000);
    loginBasic(page, { username: user.username, password });
    await waitForPromise(2000);
    const testUserProfile = await apiGetUserProfile(page, user.username);
    expect(testUserProfile.devices.length).toBe(1);
    const createdDevice = testUserProfile.devices[0];
    expect(createdDevice.networks[0].device_wireguard_ip).toBe('10.10.10.2');
    expect(createdDevice.name).toBe('test');
  });
});
