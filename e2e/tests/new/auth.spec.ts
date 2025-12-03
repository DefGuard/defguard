import { expect, test } from '@playwright/test';
import { TOTP } from 'totp-generator';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { User } from '../../types';
import { acceptRecovery } from '../../utils/controllers/acceptRecovery';
import { createUser } from '../../utils/controllers/createUser';
import { loginBasic, loginRecoveryCodes, loginTOTP } from '../../utils/controllers/login';
import { logout } from '../../utils/controllers/logout';
import { enableEmailMFA } from '../../utils/controllers/mfa/enableEmail';
import { enableTOTP } from '../../utils/controllers/mfa/enableTOTP';
import { changePassword, changePasswordByAdmin } from '../../utils/controllers/profile';
import { disableUser } from '../../utils/controllers/toggleUserState';
import { dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';
import { waitForRoute } from '../../utils/waitForRoute';

test.describe('Test user authentication', () => {
  let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });

  test('Basic auth with default admin', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await expect(page.url()).toBe(
      routes.base + routes.profile + defaultUserAdmin.username + '?tab=details',
    );
  });

  test('Create user and login as him', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, testUser);
    await expect(page.url()).toBe(
      routes.base + routes.profile + testUser.username + '?tab=details',
    );
  });

  test('Login with admin user via TOTP', async ({ page, browser }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const { secret } = await enableTOTP(browser, defaultUserAdmin);
    await loginTOTP(page, defaultUserAdmin, secret);
    await expect(page.url()).toBe(
      routes.base + routes.profile + defaultUserAdmin.username + '?tab=details',
    );
  });

  test('Login with user via TOTP', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    const { secret } = await enableTOTP(browser, testUser);
    await loginTOTP(page, testUser, secret);
    await expect(page.url()).toBe(
      routes.base + routes.profile + testUser.username + '?tab=details',
    );
  });

  test('Recovery code login', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    const { recoveryCodes } = await enableTOTP(browser, testUser);
    expect(recoveryCodes).toBeDefined();
    if (!recoveryCodes) return;
    expect(recoveryCodes?.length > 0).toBeTruthy();
    await loginRecoveryCodes(page, testUser, recoveryCodes[0]);
    await expect(page.url()).toBe(
      routes.base + routes.profile + testUser.username + '?tab=details',
    );
  });

  test('Login with Email TOTP', async ({ page, browser }) => {
    expect(true).toBe(false); // TODO: Do it when SMTP will be available to configure on dashboard / via api
    await waitForBase(page);
    await createUser(browser, testUser);
    const { secret } = await enableEmailMFA(browser, testUser);
    await loginBasic(page, testUser);
    await page.goto(routes.base + routes.auth.email);
    const { otp: code } = TOTP.generate(secret, {
      digits: 6,
      period: 60,
    });
    await page.getByTestId('field-code').fill(code);
    await page.locator('button[type="submit"]').click();
    await waitForRoute(page, routes.me);
  });

  test('Login as disabled user', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await disableUser(browser, testUser);
    await page.goto(routes.base);
    await waitForRoute(page, routes.auth.login);
    await page.getByTestId('field-username').fill(testUser.username);
    await page.getByTestId('field-password').fill(testUser.password);
    await page.getByTestId('sign-in').click();
    const responsePromise = page.waitForResponse('**/auth');
    const response = await responsePromise;
    expect(response.status()).toBe(401);
    expect(page.url()).toBe(routes.base + routes.auth.login);
  });

  test('Logout when disabled', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, testUser);
    expect(page.url()).toBe(
      routes.base + routes.profile + testUser.username + '?tab=details',
    );
    await disableUser(browser, testUser);
    const responsePromise = page.waitForResponse((resp) => resp.status() === 401);
    await page.getByTestId('avatar-icon').click();
    await page.getByTestId('logout').click();
    await responsePromise;
  });
});

test.describe('Test password change', () => {
  let testUser: User;
  const newPassword = 'MyNewPassword1!@#$';

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });

  test('Change user password', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, testUser);
    await page.getByTestId('change-password').click();
    await page.getByTestId('field-current').fill(testUser.password);
    await page.getByTestId('field-password').fill(newPassword);
    await page.getByTestId('field-repeat').fill(newPassword);
    await page.getByTestId('submit-password-change').click();
    await logout(page);
    testUser.password = newPassword;
    await loginBasic(page, testUser);
    await expect(page.url()).toBe(
      routes.base + routes.profile + testUser.username + '?tab=details',
    );
  });

  test('Change user password by admin', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.identity.users);
    const userRow = page.locator('.virtual-row').filter({ hasText: testUser.username });
    await userRow.locator('.icon-button').click();
    await page.getByTestId('change-password').click();
    await page.getByTestId('field-password').fill(newPassword);
    await page.getByTestId('submit-password-change').click();
    await logout(page);
    testUser.password = newPassword;
    await loginBasic(page, testUser);
    await expect(page.url()).toBe(
      routes.base + routes.profile + testUser.username + '?tab=details',
    );
  });
});
