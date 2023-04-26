import { expect, test } from '@playwright/test';
import totp from 'totp-generator';

import { defaultUserAdmin, routes, testsConfig } from '../config';
import { User } from '../types';
import { acceptRecovery } from '../utils/controllers/acceptRecovery';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic, loginTOTP } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { getPageClipboard } from '../utils/getPageClipboard';

test.beforeEach(async ({ page }) => {
  await page.goto(testsConfig.BASE_URL);
  await page.waitForURL(routes.auth.login, {
    waitUntil: 'networkidle',
  });
});

test('Basic auth', async ({ page }) => {
  await page.getByTestId('login-form-username').type('admin');
  await page.getByTestId('login-form-password').type('pass123');
  await page.getByTestId('login-form-submit').click();
  await page.waitForLoadState('networkidle');
  await page.waitForURL(routes.admin.wizard, {
    waitUntil: 'networkidle',
  });
  expect(page.url()).not.toBe(routes.base + routes.auth.login);
});

test('Create user and login with basic auth', async ({ page }) => {
  const testUser: User = {
    username: 'test',
    firstName: 'test first name',
    lastName: 'test last name',
    password: 'defguarD123!',
    mail: 'test@test.com',
    phone: '123456789',
  };
  await loginBasic(page, defaultUserAdmin);
  await page.waitForURL(routes.base + routes.admin.wizard);
  await createUser(page, testUser);
  await logout(page);
  await loginBasic(page, testUser);
  await page.waitForURL(routes.base + routes.me);
  expect(page.url()).toBe(routes.base + routes.me);
});

test('Login with TOTP', async ({ page }) => {
  const testUser: User = {
    username: 'test2',
    firstName: 'test first name',
    lastName: 'test last name',
    password: 'defguarD123!',
    mail: 'test@test.com',
    phone: '123456789',
  };
  await loginBasic(page, defaultUserAdmin);
  await page.waitForURL(routes.base + routes.admin.wizard);
  await createUser(page, testUser);
  await logout(page);
  await loginBasic(page, testUser);
  await page.waitForURL(routes.base + routes.me);
  expect(page.url()).toBe(routes.base + routes.me);
  await page.getByTestId('edit-user').click();
  await page.getByTestId('edit-totp').click();
  await page.getByTestId('enable-totp-option').click();
  await page.getByTestId('copy-totp').click();
  const totpURL = await getPageClipboard(page);
  expect(totpURL).toBeDefined();
  const secret = totpURL.split('secret=')[1];
  expect(secret.length).toBeGreaterThan(0);
  let token = totp(secret);
  const totpForm = page.getByTestId('register-totp-form');
  await totpForm.getByTestId('field-code').type(token);
  await totpForm.locator('button[type="submit"]').click();
  await totpForm.waitFor({ state: 'hidden' });
  await acceptRecovery(page);
  token = totp(secret);
  await loginTOTP(page, testUser, token);
  await page.waitForURL(routes.base + routes.me);
  expect(page.url()).toBe(routes.base + routes.me);
});
