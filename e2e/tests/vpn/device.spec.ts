import { expect, test } from '@playwright/test';

import { routes } from '../../config';
import { NetworkForm } from '../../types';
import { apiGetMe } from '../../utils/api/users';
import { createUser } from '../../utils/controllers/createUser';
import { loginBasic } from '../../utils/controllers/login';
import { createNetwork } from '../../utils/controllers/vpn/createNetwork';
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
  port: '5055',
};

test.describe('Add user device', () => {
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
    await loginBasic(page, testUser);
    await page.goto(routes.base + routes.me);
    await page.getByTestId('add-device').click();
    await page.getByTestId('field-name').type('test');
    await page.locator('.toggle-option').nth(0).click();
    const responsePromise = page.waitForResponse('**/device/**');
    await page.locator('form .controls button[type="submit"]').click();
    const response = await responsePromise;
    expect(response.status()).toBe(201);
    const testUserData = await apiGetMe(page);
    expect(testUserData.devices.length).toBe(1);
    const createdDevice = testUserData.devices[0];
    expect(createdDevice.wireguard_ip).toBe('10.10.10.2');
  });

  test('Add test user device with manual', async ({ page, context }) => {
    await waitForBase(page);
    const testUser = await createUser(context, 'testmanual');
    await loginBasic(page, testUser);
    await page.goto(routes.base + routes.me);
    await page.getByTestId('add-device').click();
    await page.locator('.toggle-option').nth(1).click();
    await page.getByTestId('field-name').type('test');
    await page.getByTestId('field-publicKey').type(testKeys.public);
    const responsePromise = page.waitForResponse('**/device/**');
    await page.locator('form .controls button[type="submit"]').click();
    const response = await responsePromise;
    expect(response.status()).toBe(201);
    const testUserData = await apiGetMe(page);
    expect(testUserData.devices.length).toBe(1);
    const createdDevice = testUserData.devices[0];
    expect(createdDevice.wireguard_ip).toBe('10.10.10.2');
  });
});
