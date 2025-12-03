import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { createUser } from '../../utils/controllers/createUser';
import { loginBasic } from '../../utils/controllers/login';
import { dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';
import { waitForRoute } from '../../utils/waitForRoute';
import { waitForPromise } from '../../utils/waitForPromise';

test.describe('Test groups', () => {
  test.beforeEach(() => dockerRestart());

  test('Create group', async ({ page, browser }) => {
    const group_name = 'new_group';
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.getByTestId('groups').click();
    await page.getByTestId('add-new-group').click();
    await page.getByTestId('field-name').fill(group_name);
    await page.getByTestId('next').click();
    await page.getByTestId('submit').click();
    await waitForPromise(1000);
    await expect(page.locator(':text("' + group_name + '")')).toBeVisible();
  });

  // test('Add user to admin group', async ({ page, browser }) => {
  //   const testUser = { ...testUserTemplate, username: 'test' };
  //   await waitForBase(page);
  //   await createUser(browser, testUser, ['admin']);
  //   await loginBasic(page, testUser);
  //   await waitForRoute(page, routes.admin.wizard);
  //   expect(page.url()).toBe(routes.base + routes.admin.wizard);
  // });
});
