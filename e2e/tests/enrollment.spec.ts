import { BrowserContext, expect, Page, test } from '@playwright/test';

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
import { logout } from '../utils/controllers/logout';
import { createNetwork } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../utils/docker';
import { getPageClipboard } from '../utils/getPageClipboard';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';

test.afterEach(async () => {
  dockerRestart();
});

test.describe.configure({
  mode: 'serial',
});

const testNetwork: NetworkForm = {
  name: 'test network',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  port: '5055',
};

test.describe('Create user with enrollment enabled', () => {
  let token: string;
  let page: Page;
  let context: BrowserContext;
  let user: User;

  // Setup client and user for tests
  test.beforeAll(async ({ browser }) => {
    context = await browser.newContext();
    page = await context.newPage();
    await waitForBase(page);
    user = await createUserEnrollment(context, 'testauth01');
    await createNetwork(context, testNetwork);
    logout(page);
    const response = (await getPageClipboard(page)).split('\n');
    // Extract token and url
    const tokenResponse = response[1].split(' ')[1];
    token = tokenResponse;
  });

  test.afterAll(() => {
    dockerRestart();
  });

  test('Go to enrollment', async () => {
    expect(token).toBeDefined();
    await page.goto('http://localhost:8080/');
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
    await page.getByTestId('enrollment-next').click();
    loginBasic(page, { username: user.username, password });
    await waitForPromise(2000);
    const testUserProfile = await apiGetUserProfile(page, user.username);
    expect(testUserProfile.devices.length).toBe(1);
    const createdDevice = testUserProfile.devices[0];
    expect(createdDevice.networks[0].device_wireguard_ip).toBe('10.10.10.2');
    expect(createdDevice.name).toBe('test');
  });
});
