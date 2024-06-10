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
    await disableUser(browser, user);
    expect(token).toBeDefined();
    await waitForBase(page);
    await page.goto(testsConfig.ENROLLMENT_URL);
    await waitForPromise(2000);
    // Test if we can send the token
    await selectEnrollment(page);
    await setToken(token, page);
    await expect(page.getByText('Field is invalid')).toBeVisible();
    // Check if we are still on the token page
    expect(page.url()).toBe(`${testsConfig.ENROLLMENT_URL}/token`);
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
    const deviceCreationMessage = page.waitForEvent('console');
    // Test if we can create a device configuration, if the admin has disabled us after the token validation
    await createDevice(page);
    // Creating a new device config should fail with a 400 error
    expect((await deviceCreationMessage).text()).toBe(
      'Failed to load resource: the server responded with a status of 400 (Bad Request)'
    );
    const userActivationMessage = page.waitForEvent('console');
    // Try to finish the enrollment
    await page.getByTestId('enrollment-next').click({ timeout: 2000 });
    // Activating the user should fail with a 400 error
    expect((await userActivationMessage).text()).toBe(
      'Failed to load resource: the server responded with a status of 400 (Bad Request)'
    );
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
