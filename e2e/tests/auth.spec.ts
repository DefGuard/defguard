import { expect, test } from '@playwright/test';
import { TOTP } from 'totp-generator';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { User } from '../types';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic, loginRecoveryCodes, loginTOTP } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { enableEmailMFA } from '../utils/controllers/mfa/enableEmail';
import { enableSecurityKey } from '../utils/controllers/mfa/enableSecurityKey';
import { enableTOTP } from '../utils/controllers/mfa/enableTOTP';
import { disableUser } from '../utils/controllers/toggleUserState';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForRoute } from '../utils/waitForRoute';
import { waitForPromise } from '../utils/waitForPromise';

test.describe('Test user authentication', () => {
  let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });

  test('Basic auth with default admin', async ({ page }) => {
    await waitForBase(page);
    const responsePromise = page.waitForResponse('**/auth');
    await loginBasic(page, defaultUserAdmin);
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
  });

  test('Create user and login as him', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    const responsePromise = page.waitForResponse('**/auth');
    await loginBasic(page, testUser);
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
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
    const responsePromise = page.waitForResponse('**/verify');
    await page.getByTestId('field-code').fill(code);
    await page.locator('[type="submit"]').click();
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
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
    expect(response.ok()).toBeFalsy();
  });

  test('Logout when enabled', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, testUser);
    const responsePromise = page.waitForResponse('**/logout');
    await page.getByTestId('avatar-icon').click();
    await page.getByTestId('logout').click();
    const response = await responsePromise;
    expect(response.status()).toBe(200);
    await waitForPromise(1000);
    await expect(page.url()).toBe(routes.base + routes.auth.login);
  });

  test('Logout when disabled', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, testUser);
    await disableUser(browser, testUser);
    const responsePromise = page.waitForResponse('**/logout');
    await page.getByTestId('avatar-icon').click();
    await page.getByTestId('logout').click();
    const response = await responsePromise;
    expect(response.status()).toBe(401);
  });

  test('Create user and log in with security key', async ({ page, browser, context }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    const { credentialId, rpId, privateKey, userHandle } = await enableSecurityKey(
      browser,
      testUser,
      'key_name',
    );
    await page.goto(routes.base);
    await waitForRoute(page, routes.auth.login);
    await page.getByTestId('field-username').fill(testUser.username);
    await page.getByTestId('field-password').fill(testUser.password);
    await page.getByTestId('sign-in').click();
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
    const responsePromise = page.waitForResponse('**/auth');
    await page.getByTestId('login-with-passkey').click();
    await page.waitForTimeout(2000);
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
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
    const responsePromise = page.waitForResponse('**/auth');
    await loginBasic(page, testUser);
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
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
    const responsePromise = page.waitForResponse('**/auth');
    await loginBasic(page, testUser);
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
  });
});

test.describe('API tokens management', () => {
  let testUser: User;
  const token_name = 'test token name';
  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });
  test('Add API token as default admin', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(
      routes.base + routes.profile + defaultUserAdmin.username + routes.tab.api_tokens,
    );
    await page.getByTestId('add-token').click();
    await page.getByTestId('field-name').fill(token_name);
    await page.getByTestId('submit').click();
    const api_token = await page.getByTestId('copy-field').textContent();
    await page.getByTestId('close').click();

    const row = page.locator('.table-row-container').filter({ hasText: token_name });
    await row.locator('.icon-button').click();
    await page.getByTestId('delete').click();
    await expect(row).not.toBeVisible();
    expect(api_token).toBeDefined();
  });
  test('Add API token as new user with admin privileges', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser, ['admin']);
    await loginBasic(page, testUser);
    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.api_tokens,
    );
    await page.getByTestId('add-token').click();
    await page.getByTestId('field-name').fill(token_name);
    await page.getByTestId('submit').click();
    const api_token = await page.getByTestId('copy-field').textContent();
    await page.getByTestId('close').click();

    const row = page.locator('.table-row-container').filter({ hasText: token_name });
    await row.locator('.icon-button').click();
    await page.getByTestId('delete').click();
    await expect(row).not.toBeVisible();
    expect(api_token).toBeDefined();
  });
});
