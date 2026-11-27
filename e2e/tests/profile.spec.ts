import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { User } from '../types';
import { apiCreateUser } from '../utils/api/users';
import { addUserToGroups } from '../utils/controllers/groups';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

test.describe('User profile', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    const context = await browser.newContext();
    const page = await context.newPage();
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUser(page, testUser);
    await context.close();
  });

  test('Non-admin can edit only phone number', async ({ page }) => {
    const newPhone = '987654321';
    await waitForBase(page);
    await loginBasic(page, testUser);
    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.details,
    );

    const card = page.locator('#general-card');
    await expect(card.getByTestId('field-username')).toBeDisabled();
    await expect(card.getByTestId('field-first_name')).toBeDisabled();
    await expect(card.getByTestId('field-last_name')).toBeDisabled();
    await expect(card.getByTestId('field-email')).toBeDisabled();
    await expect(card.getByTestId('field-phone')).toBeEnabled();

    await card.getByTestId('field-phone').fill(newPhone);
    const responsePromise = page.waitForResponse(
      (resp) =>
        resp.url().endsWith(`/user/${testUser.username}`) &&
        resp.request().method() === 'PUT',
    );
    await card.getByRole('button', { name: 'Save changes', exact: true }).click();
    expect((await responsePromise).status()).toBe(200);

    await page.reload();
    await expect(card.getByTestId('field-phone')).toHaveValue(newPhone);
    await expect(card.getByTestId('field-username')).toHaveValue(testUser.username);
    await expect(card.getByTestId('field-first_name')).toHaveValue(testUser.firstName);
    await expect(card.getByTestId('field-last_name')).toHaveValue(testUser.lastName);
    await expect(card.getByTestId('field-email')).toHaveValue(testUser.mail);
  });

  test('User promoted to admin can edit all fields', async ({ page, browser }) => {
    const updated = { firstName: 'Promoted first', lastName: 'Promoted last' };
    await waitForBase(page);
    await loginBasic(page, testUser);
    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.details,
    );
    const card = page.locator('#general-card');
    await expect(card.getByTestId('field-first_name')).toBeDisabled();

    await addUserToGroups(browser, testUser.username, ['admin']);

    await page.reload();
    await expect(card.getByTestId('field-username')).toBeEnabled();
    await expect(card.getByTestId('field-first_name')).toBeEnabled();
    await expect(card.getByTestId('field-last_name')).toBeEnabled();
    await expect(card.getByTestId('field-email')).toBeEnabled();

    await card.getByTestId('field-first_name').fill(updated.firstName);
    await card.getByTestId('field-last_name').fill(updated.lastName);
    const responsePromise = page.waitForResponse(
      (resp) =>
        resp.url().endsWith(`/user/${testUser.username}`) &&
        resp.request().method() === 'PUT',
    );
    await card.getByRole('button', { name: 'Save changes', exact: true }).click();
    expect((await responsePromise).status()).toBe(200);

    await page.reload();
    await expect(card.getByTestId('field-first_name')).toHaveValue(updated.firstName);
    await expect(card.getByTestId('field-last_name')).toHaveValue(updated.lastName);
  });

  test('Profile shows empty authorized apps and approval devices', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, testUser);
    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.details,
    );

    const apps = page.locator('#apps-no-data');
    await expect(apps).toContainText('No authorized apps');
    await expect(apps).toContainText(
      'Applications you allow to log in with Defguard will appear here.',
    );
    await expect(page.locator('#authorized-apps-card')).toHaveCount(0);

    const biometric = page.locator('.biometric-devices');
    await expect(biometric).toHaveClass('biometric-devices section empty');
    await expect(biometric).toContainText('Mobile App Approval Devices');
    await expect(biometric).toContainText(
      'No mobile app approval devices configured yet',
    );
    await expect(biometric.locator('.device')).toHaveCount(0);
  });
});
