import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { User } from '../types';
import { apiCreateUser } from '../utils/api/users';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

test.describe('Activity log', () => {
  const testUser: User = { ...testUserTemplate, username: 'activityuser' };

  test.beforeEach(() => dockerRestart());

  test('List and search activity log events', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUser(page, testUser);

    const rows = page.locator('#activity-log-page .virtual-row');
    const loginRow = rows.filter({ hasText: 'User login' }).filter({
      hasText: defaultUserAdmin.username,
    });
    const userAddedRow = rows.filter({ hasText: 'User added' });

    await expect(async () => {
      await page.goto(routes.base + routes.activity);
      await expect(userAddedRow).toBeVisible({ timeout: 1000 });
    }).toPass();
    await expect(loginRow.first()).toBeVisible();
    await expect(userAddedRow).toContainText(testUser.username);

    await page
      .locator('#activity-log-page')
      .getByPlaceholder('Search')
      .fill(testUser.username);
    await expect(rows.filter({ hasText: 'User login' })).toHaveCount(0);
    await expect(userAddedRow).toBeVisible();
  });
});
