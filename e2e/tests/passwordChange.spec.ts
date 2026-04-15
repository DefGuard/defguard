import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { User } from '../types';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

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
    const userRow = await page
      .locator('.virtual-row')
      .filter({ hasText: testUser.username });
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
