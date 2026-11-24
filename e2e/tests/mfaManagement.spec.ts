import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { User } from '../types';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic, loginTOTP } from '../utils/controllers/login';
import { enableEmailMFA } from '../utils/controllers/mfa/enableEmail';
import { enableSecurityKey } from '../utils/controllers/mfa/enableSecurityKey';
import { enableTOTP } from '../utils/controllers/mfa/enableTOTP';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

test.describe('MFA management', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };
  const profileUrl =
    routes.base + routes.profile + testUser.username + routes.tab.details;

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    await createUser(browser, testUser);
  });

  test('Disable TOTP', async ({ page, browser }) => {
    await waitForBase(page);
    const { secret } = await enableTOTP(browser, testUser);
    await enableEmailMFA(browser, testUser, secret);
    await loginTOTP(page, testUser, secret);
    await page.goto(profileUrl);

    const totpRow = page.getByTestId('totp-row');
    await expect(totpRow).toContainText('Enabled');
    await totpRow.locator('.icon-button').click();
    await page.locator('.menu').getByText('Disable', { exact: true }).click();
    await page.getByTestId('confirm-action-submit').click();

    await expect(totpRow).toContainText('Disabled');
    await page.reload();
    await expect(totpRow).toContainText('Disabled');
    await expect(page.getByTestId('email-codes-row')).toContainText('Enabled');
  });

  test('Disable email MFA', async ({ page, browser }) => {
    await waitForBase(page);
    const { secret } = await enableTOTP(browser, testUser);
    await enableEmailMFA(browser, testUser, secret);
    await loginTOTP(page, testUser, secret);
    await page.goto(profileUrl);

    const emailRow = page.getByTestId('email-codes-row');
    await expect(emailRow).toContainText('Enabled');
    await emailRow.locator('.icon-button').click();
    await page.locator('.menu').getByText('Disable', { exact: true }).click();
    await page.getByTestId('confirm-action-submit').click();

    await expect(emailRow).toContainText('Disabled');
    await page.reload();
    await expect(emailRow).toContainText('Disabled');
    await expect(page.getByTestId('totp-row')).toContainText('Enabled');
  });

  test('Delete security key', async ({ page, browser }) => {
    const keyName = 'e2e key';
    await waitForBase(page);
    await enableSecurityKey(browser, testUser, keyName);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(profileUrl);

    const keyRow = page.locator('[data-testid^="passkey-"]').filter({ hasText: keyName });
    await expect(keyRow).toBeVisible();
    await keyRow.locator('.icon-button').click();
    await page.locator('.menu').getByText('Delete passkey', { exact: true }).click();
    await page.getByTestId('confirm-action-submit').click();

    await expect(page.getByTestId('passkeys-empty')).toBeVisible();
    await page.reload();
    await expect(page.getByTestId('passkeys-empty')).toBeVisible();
    await expect(keyRow).toHaveCount(0);
  });

  test('Admin disables user TOTP', async ({ page, browser }) => {
    await waitForBase(page);
    const { secret } = await enableTOTP(browser, testUser);
    await enableEmailMFA(browser, testUser, secret);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(profileUrl);

    const totpRow = page.getByTestId('totp-row');
    await expect(totpRow).toContainText('Enabled');
    await totpRow.locator('.icon-button').click();
    await page.locator('.menu').getByText('Disable', { exact: true }).click();
    await page.getByTestId('confirm-action-submit').click();

    await expect(totpRow).toContainText('Disabled');
    await page.reload();
    await expect(totpRow).toContainText('Disabled');
    await expect(page.getByTestId('email-codes-row')).toContainText('Enabled');
  });

  test('Admin disables user email MFA', async ({ page, browser }) => {
    await waitForBase(page);
    const { secret } = await enableTOTP(browser, testUser);
    await enableEmailMFA(browser, testUser, secret);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(profileUrl);

    const emailRow = page.getByTestId('email-codes-row');
    await expect(emailRow).toContainText('Enabled');
    await emailRow.locator('.icon-button').click();
    await page.locator('.menu').getByText('Disable', { exact: true }).click();
    await page.getByTestId('confirm-action-submit').click();

    await expect(emailRow).toContainText('Disabled');
    await page.reload();
    await expect(emailRow).toContainText('Disabled');
    await expect(page.getByTestId('totp-row')).toContainText('Enabled');
  });

  test('Disable all MFA methods', async ({ page, browser }) => {
    await waitForBase(page);
    const { secret } = await enableTOTP(browser, testUser);
    await enableEmailMFA(browser, testUser, secret);
    await loginTOTP(page, testUser, secret);
    await page.goto(profileUrl);

    const totpRow = page.getByTestId('totp-row');
    const emailRow = page.getByTestId('email-codes-row');
    await expect(totpRow).toContainText('Enabled');
    await expect(emailRow).toContainText('Enabled');
    await page
      .locator('#profile-auth-card .header')
      .filter({ hasText: 'Multi-Factor Authentication' })
      .locator('.icon-button')
      .click();
    await page.locator('.menu').getByText('Disable all', { exact: true }).click();
    await page.getByTestId('confirm-action-submit').click();

    await expect(totpRow).toContainText('Disabled');
    await expect(emailRow).toContainText('Disabled');
    await page.reload();
    await expect(totpRow).toContainText('Disabled');
    await expect(emailRow).toContainText('Disabled');
  });

  test('Admin disables MFA for user', async ({ page, browser }) => {
    await waitForBase(page);
    const { secret } = await enableTOTP(browser, testUser);
    await enableEmailMFA(browser, testUser, secret);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.identity.users);

    const userRow = page.locator('.virtual-row').filter({ hasText: testUser.username });
    await userRow.locator('.edit-cell .icon-button').click();
    await page.locator('.menu').getByText('Disable MFA', { exact: true }).click();
    await page.getByTestId('confirm-action-submit').click();
    await expect(page.locator('#action-modal')).not.toBeVisible();

    await page.goto(profileUrl);
    await expect(page.getByTestId('totp-row')).toContainText('Disabled');
    await expect(page.getByTestId('email-codes-row')).toContainText('Disabled');

    await page.goto(routes.base + routes.identity.users);
    await userRow.locator('.edit-cell .icon-button').click();
    await expect(page.locator('.menu')).toBeVisible();
    await expect(
      page.locator('.menu').getByText('Disable MFA', { exact: true }),
    ).toHaveCount(0);
  });
});
