import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { User } from '../../types';
import { createUser } from '../../utils/controllers/createUser';
import { loginBasic } from '../../utils/controllers/login';
import { disableUser } from '../../utils/controllers/toggleUserState';
import { dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';
import { waitForRoute } from '../../utils/waitForRoute';

test.describe('Basic authentication', () => {
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
});
