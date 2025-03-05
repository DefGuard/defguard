import { expect, test } from '@playwright/test';
import { TOTP } from 'totp-generator';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { User } from '../types';
import { acceptRecovery } from '../utils/controllers/acceptRecovery';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic, loginRecoveryCodes, loginTOTP } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { enableEmailMFA } from '../utils/controllers/mfa/enableEmail';
import { enableTOTP } from '../utils/controllers/mfa/enableTOTP';
import { changePassword, changePasswordByAdmin } from '../utils/controllers/profile';
import { disableUser } from '../utils/controllers/toggleUserState';
import { dockerDown, dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';
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
    await waitForBase(page);
    await createUser(browser, testUser);
    const { secret } = await enableEmailMFA(browser, testUser);
    await loginBasic(page, testUser);
    await page.goto(routes.base + routes.auth.email);
    const { otp: code } = TOTP.generate(secret, {
      digits: 6,
      period: 60,
    });
    await page.getByTestId('field-code').type(code);
    await page.locator('button[type="submit"]').click();
    await waitForRoute(page, routes.me);
  });

  test('Login as disabled user', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await disableUser(browser, testUser);
    await page.goto(routes.base);
    await waitForRoute(page, routes.auth.login);
    await page.getByTestId('login-form-username').type(testUser.username);
    await page.getByTestId('login-form-password').type(testUser.password);
    const responsePromise = page.waitForResponse('**/auth');
    await page.getByTestId('login-form-submit').click();
    const response = await responsePromise;
    expect(response.status()).toBe(401);
    expect(page.url()).toBe(routes.base + routes.auth.login);
  });

  test('Logout when disabled', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, testUser);
    await waitForRoute(page, routes.me);
    expect(page.url()).toBe(routes.base + routes.me);
    await disableUser(browser, testUser);
    // The user should be logged out when the admin disables him
    await waitForPromise(2000);
    const responsePromise = page.waitForResponse('**/user/' + testUser.username);
    await page.locator('a[href="/me"]').click();
    const response = await responsePromise;
    expect(response.status()).toBe(401);
    expect(page.url()).toBe(routes.base + routes.auth.login);
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
    const profileURL = routes.base + routes.admin.users + '/' + testUser.username;
    await page.goto(profileURL);
    await waitForRoute(page, profileURL);
    testUser.password = await changePasswordByAdmin(page);
    await logout(page);
    await loginBasic(page, testUser);
    await waitForRoute(page, routes.me);
    expect(page.url()).toBe(routes.base + routes.me);
  });
});
