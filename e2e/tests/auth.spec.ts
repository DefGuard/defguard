import { expect, test } from '@playwright/test';
import { TOTP } from 'totp-generator';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { User } from '../types';
import { acceptRecovery } from '../utils/controllers/acceptRecovery';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic, loginRecoveryCodes, loginTOTP } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { enableEmailMFA } from '../utils/controllers/mfa/enableEmail';
import { enableSecurityKey } from '../utils/controllers/mfa/enableSecurityKey';
import { enableTOTP } from '../utils/controllers/mfa/enableTOTP';
import { changePassword, changePasswordByAdmin } from '../utils/controllers/profile';
import { disableUser } from '../utils/controllers/toggleUserState';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForRoute } from '../utils/waitForRoute';

test.describe('Test user authentication', () => {
  let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });

  test('Basic auth with default admin', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await waitForRoute(page, routes.admin.wizard);
    expect(page.url()).toBe(routes.base + routes.admin.wizard);
  });

  test('Create user and log in as him', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, testUser);
    await waitForRoute(page, routes.me);
    expect(page.url()).toBe(routes.base + routes.me);
  });

  test('Log in with admin user TOTP', async ({ page, browser }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const { secret } = await enableTOTP(browser, defaultUserAdmin);
    await acceptRecovery(page);
    await loginTOTP(page, defaultUserAdmin, secret);
    await page.waitForLoadState('networkidle');
    await waitForRoute(page, routes.admin.wizard);
  });

  test('Log in with user TOTP', async ({ page, browser }) => {
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

  test('Log in with Email TOTP', async ({ page, browser }) => {
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

  test('Log in as disabled user', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await disableUser(browser, testUser);
    await page.goto(routes.base);
    await waitForRoute(page, routes.auth.login);
    await page.getByTestId('login-form-username').fill(testUser.username);
    await page.getByTestId('login-form-password').fill(testUser.password);
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
    const responsePromise = page.waitForResponse((resp) => resp.status() === 401);
    await page.locator('a[href="/me"]').click();
    await responsePromise;
  });
  test('Disable user MFA and log in', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await enableTOTP(browser, testUser);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.admin.users, {
      waitUntil: 'networkidle',
    });
    await page.getByTestId('user-2').locator('.user-edit-cell').click();
    await page.getByTestId('disable-mfa-button').click();
    await page.waitForTimeout(800);
    await page.getByRole('button', { name: 'Disable MFA' }).click();
    await page.waitForTimeout(800);
    await page.goto(routes.base + routes.admin.users + `/${testUser.username}`, {
      waitUntil: 'networkidle',
    });
    await expect(page.locator('.mfa .status .message')).toHaveText('Disabled');
    await logout(page);
    await loginBasic(page, testUser);
    await page.waitForTimeout(800);
    await expect(page.locator('.mfa .status .message')).toHaveText('Disabled');
  });
});

test.describe('Test password change', () => {
  let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });

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

test.describe('Test security keys', () => {
  let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });

  test('Login with security key', async ({ page, browser, context }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    const { credentialId, rpId, privateKey, userHandle } = await enableSecurityKey(
      browser,
      testUser,
      'key_name',
    );
    await page.goto(routes.base);
    await waitForRoute(page, routes.auth.login);
    await page.getByTestId('login-form-username').fill(testUser.username);
    await page.getByTestId('login-form-password').fill(testUser.password);
    await page.getByTestId('login-form-submit').click();
    await page.waitForTimeout(1000);

    const authenticator = await context.newCDPSession(page);
    await authenticator.send('WebAuthn.enable');
    const { authenticatorId: loginAuthenticatorId } = await authenticator.send(
      'WebAuthn.addVirtualAuthenticator',
      {
        options: {
          protocol: 'ctap2',
          transport: 'usb',
          hasResidentKey: true,
          hasUserVerification: true,
          isUserVerified: true,
        },
      },
    );

    await authenticator.send('WebAuthn.addCredential', {
      authenticatorId: loginAuthenticatorId,
      credential: {
        credentialId,
        isResidentCredential: true,
        rpId,
        privateKey,
        userHandle,
        signCount: 1,
      },
    });
    await page.getByTestId('use-security-key').click();
    await page.waitForTimeout(2000);
    await expect(page.url()).toBe(routes.base + routes.me);
  });
});
