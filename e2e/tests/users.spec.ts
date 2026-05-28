import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { apiCreateUser } from '../utils/api/users';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

test.describe('Test users bulk actions', () => {
  test.beforeEach(() => dockerRestart());

  test('Bulk disable users', async ({ page }) => {
    const testUser = { ...testUserTemplate, username: 'testuserfirst' };
    const testUser2 = {
      ...testUserTemplate,
      username: 'testusersecond',
      mail: 'test2@test.com',
      phone: '9087654321',
    };

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUser(page, testUser);
    await apiCreateUser(page, testUser2);
    await page.goto(routes.base + routes.identity.users);

    const firstUser = page.locator('.virtual-row').filter({ hasText: testUser.username });
    const secondUser = page
      .locator('.virtual-row')
      .filter({ hasText: testUser2.username });
    await firstUser.locator('.checkbox').click();
    await secondUser.locator('.checkbox').click();

    await page.getByTestId('bulk-actions').click();
    await page.getByTestId('bulk-disable').click();
    await page
      .locator('.modal')
      .getByRole('button', { name: 'Disable', exact: true })
      .click();
    await page.locator('.modal').waitFor({ state: 'hidden' });

    await expect(firstUser).toContainText('Disabled');
    await expect(secondUser).toContainText('Disabled');
  });

  test('Bulk enable users', async ({ page }) => {
    const testUser = { ...testUserTemplate, username: 'testuserfirst' };
    const testUser2 = {
      ...testUserTemplate,
      username: 'testusersecond',
      mail: 'test2@test.com',
      phone: '9087654321',
    };

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUser(page, testUser);
    await apiCreateUser(page, testUser2);
    await page.goto(routes.base + routes.identity.users);

    const firstUser = page.locator('.virtual-row').filter({ hasText: testUser.username });
    const secondUser = page
      .locator('.virtual-row')
      .filter({ hasText: testUser2.username });

    await firstUser.locator('.checkbox').click();
    await secondUser.locator('.checkbox').click();
    await page.getByTestId('bulk-actions').click();
    await page.getByTestId('bulk-disable').click();
    await page
      .locator('.modal')
      .getByRole('button', { name: 'Disable', exact: true })
      .click();
    await page.locator('.modal').waitFor({ state: 'hidden' });
    await expect(firstUser).toContainText('Disabled');
    await expect(secondUser).toContainText('Disabled');

    // Row selection persists across bulk actions; re-clicking checkboxes would deselect.
    await page.getByTestId('bulk-actions').click();
    await page.getByTestId('bulk-enable').click();
    await page
      .locator('.modal')
      .getByRole('button', { name: 'Enable', exact: true })
      .click();
    await page.locator('.modal').waitFor({ state: 'hidden' });

    await expect(firstUser).toContainText('Active');
    await expect(secondUser).toContainText('Active');
  });

  test('Bulk delete users', async ({ page }) => {
    const testUser = { ...testUserTemplate, username: 'testuserfirst' };
    const testUser2 = {
      ...testUserTemplate,
      username: 'testusersecond',
      mail: 'test2@test.com',
      phone: '9087654321',
    };

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUser(page, testUser);
    await apiCreateUser(page, testUser2);
    await page.goto(routes.base + routes.identity.users);

    const firstUser = page.locator('.virtual-row').filter({ hasText: testUser.username });
    const secondUser = page
      .locator('.virtual-row')
      .filter({ hasText: testUser2.username });
    await expect(firstUser).toBeVisible();
    await expect(secondUser).toBeVisible();

    await firstUser.locator('.checkbox').click();
    await secondUser.locator('.checkbox').click();
    await page.getByTestId('bulk-actions').click();
    await page.getByTestId('bulk-delete').click();
    await page
      .locator('.modal')
      .getByRole('button', { name: 'Delete', exact: true })
      .click();
    await page.locator('.modal').waitFor({ state: 'hidden' });

    await expect(firstUser).toHaveCount(0);
    await expect(secondUser).toHaveCount(0);
  });
});
