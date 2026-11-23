import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testsConfig, testUserTemplate } from '../config';
import { User } from '../types';
import { addApiToken } from '../utils/controllers/apiToken';
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

  test('Rename API token', async ({ page, browser }) => {
    const newName = 'renamed token';
    await waitForBase(page);
    await addApiToken(browser, defaultUserAdmin, token_name);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(
      routes.base + routes.profile + defaultUserAdmin.username + routes.tab.api_tokens,
    );

    const row = page.locator('.table-row-container').filter({ hasText: token_name });
    await row.locator('.icon-button').click();
    await page.getByTestId('edit').click();
    const modal = page.locator('.modal');
    await modal.getByTestId('field-name').fill(newName);
    const responsePromise = page.waitForResponse('**/api_token/*/rename');
    await modal.getByRole('button', { name: 'Save changes', exact: true }).click();
    expect((await responsePromise).status()).toBe(200);
    await modal.waitFor({ state: 'hidden' });

    await page.reload();
    await expect(
      page.locator('.table-row-container').filter({ hasText: newName }),
    ).toBeVisible();
    await expect(
      page.locator('.table-row-container').filter({ hasText: token_name }),
    ).toHaveCount(0);
  });

  test('Deleted API token no longer authorizes requests', async ({
    page,
    browser,
    request,
  }) => {
    await waitForBase(page);
    const apiToken = await addApiToken(browser, defaultUserAdmin, token_name);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(
      routes.base + routes.profile + defaultUserAdmin.username + routes.tab.api_tokens,
    );
    const authHeaders = { Authorization: `Bearer ${apiToken}` };

    const meUrl = testsConfig.CORE_BASE_URL + '/me';
    expect((await request.get(meUrl, { headers: authHeaders })).status()).toBe(200);

    const row = page.locator('.table-row-container').filter({ hasText: token_name });
    await row.locator('.icon-button').click();
    await page.getByTestId('delete').click();
    await page.locator('button[data-variant="critical"]').click();
    await expect(row).not.toBeVisible();

    expect((await request.get(meUrl, { headers: authHeaders })).status()).toBe(401);
  });

  test('Empty state for admin and no tab for non-admin', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(
      routes.base + routes.profile + defaultUserAdmin.username + routes.tab.api_tokens,
    );
    await expect(page.getByText("You don't have any API tokens.")).toBeVisible();
    await expect(page.getByTestId('add-token')).toBeVisible();

    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.details,
    );
    await expect(page.locator('#general-card')).toBeVisible();
    await expect(page.getByText('API Tokens', { exact: true })).toHaveCount(0);

    const context = await browser.newContext();
    const userPage = await context.newPage();
    await loginBasic(userPage, testUser);
    await userPage.goto(
      routes.base + routes.profile + testUser.username + routes.tab.api_tokens,
    );
    await userPage.waitForURL('**' + routes.tab.details);
    await expect(userPage.locator('#general-card')).toBeVisible();
    await expect(userPage.getByText('API Tokens', { exact: true })).toHaveCount(0);
    await expect(userPage.getByTestId('add-token')).toHaveCount(0);
    await context.close();
  });
});
