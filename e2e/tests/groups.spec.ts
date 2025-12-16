import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { apiCreateUser } from '../utils/api/users';
import { createUser } from '../utils/controllers/createUser';
import { createGroup } from '../utils/controllers/groups';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';

test.describe('Test groups', () => {
  test.beforeEach(() => dockerRestart());

  test('Create group', async ({ page, browser }) => {
    const groups = ['test_group1', 'test_group2', 'test_group3'];

    for (const group of groups) {
      await createGroup(browser, group);
    }
    await loginBasic(page, defaultUserAdmin);
    await waitForPromise(1000);
    await page.goto(routes.base + routes.identity.groups);
    for (const group of groups) {
      await expect(page.locator('text=' + group + '')).toBeVisible();
    }
  });

  test('Add user to admin group', async ({ page, browser }) => {
    const testUser = { ...testUserTemplate, username: 'test' };
    await waitForBase(page);
    await createUser(browser, testUser, ['admin']);
    await loginBasic(page, testUser);
    await page.goto(routes.base + routes.openid_apps);
    const addButton = page.getByTestId('add-new-app');
    await expect(addButton).toBeVisible();
    await expect(addButton).toBeEnabled();
  });
  test('Add user to new group', async ({ page, browser }) => {
    const testUser = { ...testUserTemplate, username: 'test' };
    await waitForBase(page);
    await createGroup(browser, 'test_group2');
    await createUser(browser, testUser, ['test_group2']);
    await loginBasic(page, testUser);
    await expect(page.url()).toBe(
      routes.base + routes.profile + testUser.username + routes.tab.details,
    );
  });
  test('Bulk assign users to new group', async ({ page, browser }) => {
    const testUser = { ...testUserTemplate, username: 'testuserfirst' };
    const testUser2 = { ...testUserTemplate, username: 'testusersecond' };
    const group_name = 'test_group2';
    testUser2.mail = 'test2@test.com';
    testUser2.phone = '9087654321';

    await waitForBase(page);
    await createGroup(browser, group_name);
    await loginBasic(page, defaultUserAdmin);

    await apiCreateUser(page, testUser);
    await apiCreateUser(page, testUser2);
    await page.goto(routes.base + routes.identity.users);
    const firstUser = await page
      .locator('.virtual-row')
      .filter({ hasText: testUser.username });
    await firstUser.locator('.checkbox').click();
    const secondUser = await page
      .locator('.virtual-row')
      .filter({ hasText: testUser2.username });
    await secondUser.locator('.checkbox').click();
    await page.getByTestId('bulk-assign').click();
    await page
      .locator('.modal')
      .locator('.checkbox')
      .filter({ hasText: group_name })
      .click();
    await page.getByTestId('submit').click();
    await waitForPromise(2000);
    await page.goto(routes.base + routes.identity.users);

    await expect(firstUser).toContainText(group_name);
    await expect(secondUser).toContainText(group_name);
  });
});
