import { expect, test } from '@playwright/test';
import totp from 'totp-generator';

import { defaultUserAdmin, routes } from '../config';
import { User } from '../types';
import { acceptRecovery } from '../utils/controllers/acceptRecovery';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic, loginTOTP } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { getPageClipboard } from '../utils/getPageClipboard';
import { waitForRoute } from '../utils/waitForRoute';

test('Basic auth', async ({ page }) => {
  await loginBasic(page, defaultUserAdmin);
  await waitForRoute(page, routes.admin.wizard);
  expect(page.url()).toBe(routes.base + routes.admin.wizard);
});

test('Create user and login', async ({ page }) => {
  const testUser: User = {
    username: 'test',
    firstName: 'test first name',
    lastName: 'test last name',
    password: 'defguarD123!',
    mail: 'test@test.com',
    phone: '123456789',
  };
  await loginBasic(page, defaultUserAdmin);
  await waitForRoute(page, routes.admin.wizard);
  await createUser(page, testUser);
  await logout(page);
  await waitForRoute(page, routes.auth.login);
  await loginBasic(page, testUser);
  await waitForRoute(page, routes.me);
  expect(page.url()).toBe(routes.base + routes.me);
});

test('Login with TOTP', async ({ page }) => {
  const testUser: User = {
    username: 'testtotp',
    firstName: 'test first name',
    lastName: 'test last name',
    password: 'defguarD123!',
    mail: 'test@test.com',
    phone: '123456789',
  };
  await loginBasic(page, defaultUserAdmin);
  await waitForRoute(page, routes.admin.wizard);
  await createUser(page, testUser);
  await logout(page);
  await loginBasic(page, testUser);
  await waitForRoute(page, routes.me);
  await page.getByTestId('edit-user').click();
  await page.getByTestId('edit-totp').click();
  await page.getByTestId('enable-totp-option').click();
  await page.getByTestId('copy-totp').click();
  const totpURL = await getPageClipboard(page);
  expect(totpURL).toBeDefined();
  const secret = totpURL.split('secret=')[1];
  expect(secret.length).toBeGreaterThan(0);
  const token = totp(secret);
  const totpForm = page.getByTestId('register-totp-form');
  await totpForm.getByTestId('field-code').type(token);
  await totpForm.locator('button[type="submit"]').click();
  await totpForm.waitFor({ state: 'hidden' });
  await acceptRecovery(page);
  await loginTOTP(page, testUser, secret);
  await waitForRoute(page, routes.me);
  await page.waitForURL('**' + routes.me);
  expect(page.url()).toBe(routes.base + routes.me);
});
