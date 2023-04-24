import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testsConfig } from '../config';
import { User } from '../types';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic } from '../utils/controllers/loginBasic';
import { logout } from '../utils/controllers/logout';

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
  expect(page.url()).toBe(routes.base + routes.admin.wizard);
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
  await createUser(page, testUser);
  await logout(page);
  await loginBasic(page, testUser);
  expect(page.url()).toBe(routes.base + routes.admin.wizard);
});
