import { expect, test } from '@playwright/test';

import { routes, testsConfig } from '../config';

test.beforeEach(async ({ page }) => {
  await page.goto(testsConfig.BASE_URL);
});

test('Basic auth', async ({ page }) => {
  await page.waitForURL(routes.auth.login, {
    waitUntil: 'networkidle',
  });
  await page.getByTestId('login-form-username').type('admin');
  await page.getByTestId('login-form-password').type('pass123');
  await page.getByTestId('login-form-submit').click();
  await page.waitForLoadState('networkidle');
  await page.waitForURL(routes.admin.wizard, {
    waitUntil: 'networkidle',
  });
  const url = page.url();
  expect(url).toBe(routes.base + routes.admin.wizard + '/');
});
