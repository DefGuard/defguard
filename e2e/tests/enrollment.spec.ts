import { expect, test } from '@playwright/test';

import { testsConfig, testUserTemplate } from '../config';
import { NetworkForm, User } from '../types';
import { apiGetUserProfile } from '../utils/api/users';
import {
  createDevice,
  createUserEnrollment,
  password,
  selectEnrollment,
  setPassword,
  setToken,
  validateData,
} from '../utils/controllers/enrollment';
import { loginBasic } from '../utils/controllers/login';
import { disableUser, enableUser } from '../utils/controllers/toggleUserState';
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

  test('Try to complete enrollment with disabled user', async ({ page, browser }) => {
    expect(token).toBeDefined();
    await waitForBase(page);
    await disableUser(browser, user);
    await page.goto(testsConfig.ENROLLMENT_URL);
    await waitForPromise(2000);
    // Test if we can send the token
    await selectEnrollment(page);
    const startResponse = page.waitForResponse('**/start');
    await setToken(token, page);
    expect((await startResponse).status()).toBe(403);
    // Check if we are still on the token page
    expect(page.url()).toBe(`${testsConfig.ENROLLMENT_URL}/token`);

    // Test other enrollment steps
    await enableUser(browser, user);
    await page.reload();
    await setToken(token, page);
    // Welcome page
    await page.getByTestId('enrollment-next').click();
    // Data validation
    await validateData(user, page);
    await page.getByTestId('enrollment-next').click();
    await disableUser(browser, user);
    // Set password
    await setPassword(page);
    // VPN
    await page.getByTestId('enrollment-next').click();

    // Test if we can create a device configuration, if the admin has disabled us after the token validation
    const deviceResponse = page.waitForResponse('**/create_device');
    await createDevice(page);
    expect((await deviceResponse).status()).toBe(400);

    // Activating the user should fail with a 400 error
    const userResponse = page.waitForResponse('**/activate_user');
    await page.getByTestId('enrollment-next').click({ timeout: 2000 });
    expect((await userResponse).status()).toBe(400);
  });

  test('Complete enrollment with created user', async ({ page }) => {
    expect(token).toBeDefined();
    await waitForBase(page);
    await page.goto(testsConfig.ENROLLMENT_URL);
    await waitForPromise(2000);
    await selectEnrollment(page);
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
