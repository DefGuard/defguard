import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { createUser } from '../../utils/controllers/createUser';
import { loginBasic } from '../../utils/controllers/login';
import { dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';
import { createGroup } from '../../utils/controllers/groups';
import { waitForPromise } from '../../utils/waitForPromise';

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
      routes.base + routes.profile + testUser.username + '?tab=details',
    );
  });
});
