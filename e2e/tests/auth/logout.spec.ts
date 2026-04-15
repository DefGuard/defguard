import { expect, test } from '@playwright/test';

import { testUserTemplate } from '../../config';
import { User } from '../../types';
import { createUser } from '../../utils/controllers/createUser';
import { loginBasic } from '../../utils/controllers/login';
import { logout } from '../../utils/controllers/logout';
import { disableUser } from '../../utils/controllers/toggleUserState';
import { dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';
import { waitForPromise } from '../../utils/waitForPromise';
import { routes } from '../../config';

test.describe('Logout', () => {
  let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });

  test('Logout when enabled', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, testUser);
    const responsePromise = page.waitForResponse('**/logout');
    await logout(page);
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
    await logout(page);
    const response = await responsePromise;
    expect(response.status()).toBe(401);
  });
});
