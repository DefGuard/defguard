import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { apiCreateUser } from '../utils/api/users';
import { createUser } from '../utils/controllers/createUser';
import { createGroup } from '../utils/controllers/groups';
import { loginBasic } from '../utils/controllers/login';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

test.describe('Test groups', () => {
  test.beforeEach(() => dockerRestart());

  test('Create group', async ({ page, browser }) => {
    const groups = ['test_group1', 'test_group2', 'test_group3'];

    for (const group of groups) {
      await createGroup(browser, group);
    }
    await loginBasic(page, defaultUserAdmin);
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
    // Use toHaveURL (retries) not expect(page.url()).toBe (synchronous, no retry).
    await expect(page).toHaveURL(
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
    const firstUser = page.locator('.virtual-row').filter({ hasText: testUser.username });
    await firstUser.locator('.checkbox').click();
    const secondUser = page
      .locator('.virtual-row')
      .filter({ hasText: testUser2.username });
    await secondUser.locator('.checkbox').click();

    await page.getByTestId('users-bulk-actions').click();
    await page.getByTestId('users-bulk-assign-to-group').click();
    await page
      .locator('.modal')
      .locator('.checkbox')
      .filter({ hasText: group_name })
      .click();
    await page.locator('.modal').getByTestId('submit').click();
    await page.locator('.modal').waitFor({ state: 'hidden' });
    await page.goto(routes.base + routes.identity.users);

    await expect(firstUser).toContainText(group_name);
    await expect(secondUser).toContainText(group_name);
  });
  test('Rename group', async ({ page, browser }) => {
    const group = 'rename_me';
    const newName = 'renamed_group';
    await waitForBase(page);
    await createGroup(browser, group);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.identity.groups);

    const row = page.locator('.virtual-row').filter({ hasText: group });
    await row.locator('.icon-button').click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    const modal = page.locator('#ce-group-modal');
    await modal.waitFor({ state: 'visible' });
    await modal.getByTestId('field-name').fill(newName);
    await modal.getByTestId('next').click();
    await modal.getByTestId('submit').click();
    await modal.waitFor({ state: 'hidden' });

    await page.reload();
    await expect(page.locator('.virtual-row').filter({ hasText: newName })).toBeVisible();
    await expect(page.locator('.virtual-row').filter({ hasText: group })).toHaveCount(0);
  });

  test('Set group as admin group', async ({ page, browser }) => {
    const group = 'future_admins';
    await waitForBase(page);
    await createGroup(browser, group);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.identity.groups);

    const row = page.locator('.virtual-row').filter({ hasText: group });
    await expect(row).toContainText('User');
    await row.locator('.icon-button').click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    const modal = page.locator('#ce-group-modal');
    await modal.waitFor({ state: 'visible' });
    await modal
      .locator('.checkbox')
      .filter({ hasText: 'Set this group as Admin group' })
      .click();
    await modal.getByTestId('next').click();
    await modal.getByTestId('submit').click();
    await modal.waitFor({ state: 'hidden' });

    await page.reload();
    await expect(row).toContainText('Admin');
  });

  test('Edit group members', async ({ page, browser }) => {
    const group = 'members_group';
    const testUser = { ...testUserTemplate, username: 'test' };
    const memberLabel = `${testUser.firstName} ${testUser.lastName}`;
    await waitForBase(page);
    await createUser(browser, testUser);
    await createGroup(browser, group);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.identity.groups);

    const row = page.locator('.virtual-row').filter({ hasText: group });
    await row.locator('.icon-button').click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    const modal = page.locator('#ce-group-modal');
    await modal.waitFor({ state: 'visible' });
    await modal.getByTestId('next').click();
    await modal
      .locator('.item')
      .filter({ hasText: memberLabel })
      .locator('.checkbox')
      .click();
    await modal.getByTestId('submit').click();
    await modal.waitFor({ state: 'hidden' });

    await page.goto(routes.base + routes.identity.users);
    const userRow = page.locator('.virtual-row').filter({ hasText: testUser.username });
    await expect(userRow).toContainText(group);

    await page.goto(routes.base + routes.identity.groups);
    await row.locator('.icon-button').click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    await modal.waitFor({ state: 'visible' });
    await modal.getByTestId('next').click();
    await modal
      .locator('.item')
      .filter({ hasText: memberLabel })
      .locator('.checkbox')
      .click();
    await modal.getByTestId('submit').click();
    await modal.waitFor({ state: 'hidden' });

    await page.goto(routes.base + routes.identity.users);
    await expect(userRow).not.toContainText(group);
  });

  test('Delete group', async ({ page, browser }) => {
    const group = 'delete_me';
    await waitForBase(page);
    await createGroup(browser, group);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.identity.groups);

    const row = page.locator('.virtual-row').filter({ hasText: group });
    await row.locator('.icon-button').click();
    await page.locator('.menu').getByText('Delete', { exact: true }).click();
    await page.getByTestId('confirm-action-submit').click();
    await expect(row).toHaveCount(0);

    await page.reload();
    await expect(page.locator('.virtual-row').filter({ hasText: 'admin' })).toBeVisible();
    await expect(row).toHaveCount(0);
  });

  test('Delete group blocked when it is the only allowed group in a location', async ({
    page,
    browser,
  }) => {
    const group = 'only_allowed';
    await waitForBase(page);
    await createGroup(browser, group);
    await createRegularLocation(browser, {
      name: 'restricted location',
      address: '10.10.10.1/24',
      endpoint: '127.0.0.1',
      port: '5055',
      allowedGroups: [group],
    });
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.identity.groups);

    const row = page.locator('.virtual-row').filter({ hasText: group });
    await expect(row).toContainText('restricted location');
    await row.locator('.icon-button').click();
    await page.locator('.menu').getByText('Delete', { exact: true }).click();
    const modal = page.locator('#action-modal');
    await expect(modal).toContainText('is the only allowed group in location');
    await expect(modal.getByTestId('confirm-action-submit')).toHaveCount(0);
    await modal.getByTestId('confirm-action-cancel').click();
    await modal.waitFor({ state: 'hidden' });

    await page.reload();
    await expect(row).toBeVisible();
  });
});
