import { expect, test } from '@playwright/test';
import totp from 'totp-generator';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { User } from '../types';
import { acceptRecovery } from '../utils/controllers/acceptRecovery';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic, loginRecoveryCodes, loginTOTP } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { enableEmailMFA } from '../utils/controllers/mfa/enableEmail';
import { enableTOTP } from '../utils/controllers/mfa/enableTOTP';
import { changePassword, changePasswordByAdmin } from '../utils/controllers/profile';
import { dockerDown, dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForRoute } from '../utils/waitForRoute';

test.describe('Test user authentication', () => {
  let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });

  test.afterAll(() => dockerDown());

  test('Basic auth with default admin', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await waitForRoute(page, routes.admin.wizard);
    expect(page.url()).toBe(routes.base + routes.admin.wizard);
  });

  test('Create user and login as him', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, testUser);
    await waitForRoute(page, routes.me);
    expect(page.url()).toBe(routes.base + routes.me);
  });

  test('Login with admin user TOTP', async ({ page, browser }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const { secret } = await enableTOTP(browser, defaultUserAdmin);
    await acceptRecovery(page);
    await loginTOTP(page, defaultUserAdmin, secret);
    await page.waitForLoadState('networkidle');
    await waitForRoute(page, routes.admin.wizard);
  });

  test('Login with user TOTP', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    const { secret } = await enableTOTP(browser, testUser);
    await loginTOTP(page, testUser, secret);
    await waitForRoute(page, routes.me);
    expect(page.url()).toBe(routes.base + routes.me);
  });

  test('Recovery code login', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    const { recoveryCodes } = await enableTOTP(browser, testUser);
    expect(recoveryCodes).toBeDefined();
    if (!recoveryCodes) return;
    expect(recoveryCodes?.length > 0).toBeTruthy();
    await loginRecoveryCodes(page, testUser, recoveryCodes[0]);
    await waitForRoute(page, routes.me);
    expect(page.url()).toBe(routes.base + routes.me);
  });

  test('Login with Email TOTP', async ({ page, browser }) => {
    test.skip(true, 'Make it later');
    await waitForBase(page);
    await createUser(browser, testUser);
    const { secret } = await enableEmailMFA(browser, testUser);
    await loginBasic(page, testUser);
    await page.goto(routes.base + routes.auth.email);
    const code = totp(secret);
    await page.getByTestId('field-code').type(code);
    await page.locator('button[type="submit"]').click();
    await waitForRoute(page, routes.me);
  });
});

test.describe('Test password change', () => {
  let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });

  test.afterAll(() => dockerDown());

  test('Change user password', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, testUser);
    testUser.password = await changePassword(page, testUser.password);
    await logout(page);
    await loginBasic(page, testUser);
    await waitForRoute(page, routes.me);
    expect(page.url()).toBe(routes.base + routes.me);
  });

  test('Change user password by admin', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.admin.users);
    await page.getByText(testUser.username, { exact: true }).click();
    testUser.password = await changePasswordByAdmin(page);
    await logout(page);
    await loginBasic(page, testUser);
    await waitForRoute(page, routes.me);
    expect(page.url()).toBe(routes.base + routes.me);
  });
});
