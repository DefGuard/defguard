import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { createUser } from '../utils/controllers/createUser';
import { createGroup } from '../utils/controllers/groups';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';
import { waitForRoute } from '../utils/waitForRoute';

test.describe('Test groups', () => {
  test.beforeEach(() => dockerRestart());

  test('Add user to admin group', async ({ page, browser }) => {
    const testUser = { ...testUserTemplate, username: 'test' };
    await waitForBase(page);
    await createUser(browser, testUser, ['admin']);
    await loginBasic(page, testUser);
    await waitForRoute(page, routes.admin.wizard);
    expect(page.url()).toBe(routes.base + routes.admin.wizard);
  });

  test('Bulk assign groups', async ({ page, browser }) => {
    const additionalUsers = [
      { ...testUserTemplate, mail: 'test2@test.com', username: 'test2' },
      { ...testUserTemplate, mail: 'test3@test.com', username: 'test3' },
    ];
    const test_group_name = 'test_group';
    await waitForBase(page);
    const testUser1 = { ...testUserTemplate, mail: 'test1@test.com', username: 'test1' };
    await createUser(browser, testUser1, ['Admin']);

    for (const newuser of additionalUsers) {
      await createUser(browser, newuser);
    }
    await createGroup(browser, true, test_group_name);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.admin.users, {
      waitUntil: 'networkidle',
    });

    const userCheckboxes = page.locator('[data-testid^="user-"]').locator('.select-cell');
    const checkboxCount = await page.locator('[data-testid^="user-"]').count();
    for (let i = 0; i < checkboxCount; i++) {
      await userCheckboxes.nth(i).click();
    }
    await page.getByTestId('group-bulk-assign').click();
    await page.locator('.groups-container').waitFor({ state: 'visible', timeout: 10000 });
    await waitForPromise(2000);
    await page.locator('.select-row').nth(2).click();
    await page.getByTestId('confirm-bulk-assign').click();
    await page.locator('.groups-container').waitFor({ state: 'hidden', timeout: 10000 });
    const testGroupElements = page.locator(
      `.groups-cell .group .text-container:has-text("${test_group_name}")`,
    );

    // 2(default admin + user with admin group) + additional users
    await expect(testGroupElements).toHaveCount(2 + additionalUsers.length, {
      timeout: 5000,
    });
  });
});
