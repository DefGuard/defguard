import { expect, test } from '@playwright/test';
import { faker } from '@faker-js/faker';

import { defaultUserAdmin, routes } from '../config';
import { acceptRecovery } from '../utils/controllers/acceptRecovery';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic, loginRecoveryCodes, loginTOTP } from '../utils/controllers/login';
import { enableTOTP } from '../utils/controllers/mfa/enableTOTP';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForRoute } from '../utils/waitForRoute';

test.afterEach(async () => {
  dockerRestart();
});

test('Basic auth with default admin', async ({ page }) => {
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await waitForRoute(page, routes.admin.wizard);
  expect(page.url()).toBe(routes.base + routes.admin.wizard);
});

test('Create user and login as him', async ({ page, context }) => {
  await waitForBase(page);
  const testUser = await createUser(context, 'testauth01');
  await loginBasic(page, testUser);
  await waitForRoute(page, routes.me);
  expect(page.url()).toBe(routes.base + routes.me);
});

test('Login with admin user TOTP', async ({ page, context }) => {
  await waitForBase(page);
  const testUser = await createUser(context, 'testtotp1', ['Admin']);
  await loginBasic(page, testUser);
  const secret = await enableTOTP(page);
  await acceptRecovery(page);
  await loginTOTP(page, testUser, secret);
  await page.waitForLoadState('networkidle');
  await waitForRoute(page, routes.admin.wizard);
});

test('Login with user TOTP', async ({ page, context }) => {
  await waitForBase(page);
  const testUser = await createUser(context, 'testtotp2');
  await loginBasic(page, testUser);
  await waitForRoute(page, routes.me);
  const secret = await enableTOTP(page);
  await acceptRecovery(page);
  await loginTOTP(page, testUser, secret);
  await waitForRoute(page, routes.me);
  expect(page.url()).toBe(routes.base + routes.me);
});

test('Recovery code login', async ({ page, context }) => {
  await waitForBase(page);
  const testUser = await createUser(context, 'recovery');
  await loginBasic(page, testUser);
  await waitForRoute(page, routes.me);
  await enableTOTP(page);
  const recoveryCodes = await acceptRecovery(page);
  await waitForRoute(page, routes.auth.login);
  await loginRecoveryCodes(page, testUser, recoveryCodes[0]);
  await waitForRoute(page, routes.me);
  expect(page.url()).toBe(routes.base + routes.me);
});

test('Add user to admin group', async ({ page, context }) => {
  await waitForBase(page);
  const testUser = await createUser(context, faker.person.lastName().toLowerCase(), [
    'Admin',
  ]);
  await loginBasic(page, testUser);
  await waitForRoute(page, routes.admin.wizard);
  expect(page.url()).toBe(routes.base + routes.admin.wizard);
});
