import { expect, test } from '@playwright/test';

import { defaultUserAdmin, testUserTemplate } from '../../config';
import { User } from '../../types';
import { createUser } from '../../utils/controllers/createUser';
import { loginBasic, loginRecoveryCodes, loginTOTP } from '../../utils/controllers/login';
import { enableTOTP } from '../../utils/controllers/mfa/enableTOTP';
import { dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';

test.describe('TOTP authentication', () => {
  let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });

  test('Login with admin user via TOTP', async ({ page, browser }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const { secret } = await enableTOTP(browser, defaultUserAdmin);
    const responsePromise = page.waitForResponse('**/auth');
    await loginTOTP(page, defaultUserAdmin, secret);
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
  });

  test('Login with user via TOTP', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    const { secret } = await enableTOTP(browser, testUser);
    const responsePromise = page.waitForResponse('**/auth');
    await loginTOTP(page, testUser, secret);
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
  });

  test('Recovery code login', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    const { recoveryCodes } = await enableTOTP(browser, testUser);
    expect(recoveryCodes).toBeDefined();
    if (!recoveryCodes) return;
    expect(recoveryCodes?.length > 0).toBeTruthy();
    const responsePromise = page.waitForResponse('**/auth');
    await loginRecoveryCodes(page, testUser, recoveryCodes[0]);
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
  });
});
