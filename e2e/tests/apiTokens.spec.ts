import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { User } from '../types';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

test.describe('API tokens management', () => {
  let testUser: User;
  const token_name = 'test token name';

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });

  test('Add API token as default admin', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(
      routes.base + routes.profile + defaultUserAdmin.username + routes.tab.api_tokens,
    );
    await page.getByTestId('add-token').click();
    await page.getByTestId('field-name').fill(token_name);
    await page.getByTestId('submit').click();
    const api_token = await page.getByTestId('copy-field').textContent();
    await page.getByTestId('close').click();

    const row = await page
      .locator('.table-row-container')
      .filter({ hasText: token_name });
    await row.locator('.icon-button').click();
    await page.getByTestId('delete').click();
    await page.locator('button[data-variant="critical"]').click();
    await expect(row).not.toBeVisible();
    expect(api_token).toBeDefined();
  });

  test('Add API token as new user with admin privileges', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser, ['admin']);
    await loginBasic(page, testUser);
    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.api_tokens,
    );
    await page.getByTestId('add-token').click();
    await page.getByTestId('field-name').fill(token_name);
    await page.getByTestId('submit').click();
    const api_token = await page.getByTestId('copy-field').textContent();
    await page.getByTestId('close').click();

    const row = await page
      .locator('.table-row-container')
      .filter({ hasText: token_name });
    await row.locator('.icon-button').click();
    await page.getByTestId('delete').click();
    await page.locator('button[data-variant="critical"]').click();
    await expect(row).not.toBeVisible();
    expect(api_token).toBeDefined();
  });
});
