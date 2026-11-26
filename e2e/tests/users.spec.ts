import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testsConfig, testUserTemplate } from '../config';
import {
  apiEnrollmentActivateUser,
  apiEnrollmentCreateDevice,
  apiEnrollmentStart,
} from '../utils/api/enrollment';
import { apiClearMailbox, apiWaitForMail } from '../utils/api/mailpit';
import { apiCreateUser, apiLogin } from '../utils/api/users';
import { createUserEnrollment } from '../utils/controllers/enrollment';
import { loginBasic } from '../utils/controllers/login';
import { setupSMTP } from '../utils/controllers/settings';
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

    await page.getByTestId('users-bulk-actions').click();
    await page.getByTestId('users-bulk-disable').click();
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
    await page.getByTestId('users-bulk-actions').click();
    await page.getByTestId('users-bulk-disable').click();
    await page
      .locator('.modal')
      .getByRole('button', { name: 'Disable', exact: true })
      .click();
    await page.locator('.modal').waitFor({ state: 'hidden' });
    await expect(firstUser).toContainText('Disabled');
    await expect(secondUser).toContainText('Disabled');

    // Row selection persists across bulk actions; re-clicking checkboxes would deselect.
    await page.getByTestId('users-bulk-actions').click();
    await page.getByTestId('users-bulk-enable').click();
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
    await page.getByTestId('users-bulk-actions').click();
    await page.getByTestId('users-bulk-delete').click();
    await page
      .locator('.modal')
      .getByRole('button', { name: 'Delete', exact: true })
      .click();
    await page.locator('.modal').waitFor({ state: 'hidden' });

    await expect(firstUser).toHaveCount(0);
    await expect(secondUser).toHaveCount(0);
  });

  test('Delete single user', async ({ page, request }) => {
    const testUser = { ...testUserTemplate, username: 'testuserfirst' };

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUser(page, testUser);
    expect(await apiLogin(request, testUser.username, testUser.password)).toBe(200);
    await page.goto(routes.base + routes.identity.users);

    const userRow = page.locator('.virtual-row').filter({ hasText: testUser.username });
    await userRow.locator('.icon-button').click();
    await page.locator('.menu').getByText('Delete account', { exact: true }).click();
    await page
      .locator('.modal')
      .getByRole('button', { name: 'Delete account', exact: true })
      .click();
    await page.locator('.modal').waitFor({ state: 'hidden' });

    await expect(userRow).toHaveCount(0);
    expect(await apiLogin(request, testUser.username, testUser.password)).toBe(401);
  });

  test('Edit user details', async ({ page }) => {
    const testUser = { ...testUserTemplate, username: 'testuserfirst' };
    const updated = {
      username: 'testuseredited',
      mail: 'edited@test.com',
      firstName: 'Edited first',
      lastName: 'Edited last',
      phone: '555444333',
    };

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUser(page, testUser);
    await page.goto(routes.base + routes.identity.users);

    const userRow = page.locator('.virtual-row').filter({ hasText: testUser.username });
    await userRow.locator('.icon-button').click();
    await page.locator('.menu').getByText('Edit details', { exact: true }).click();
    const modal = page.locator('#edit-user-modal');
    await modal.waitFor({ state: 'visible' });
    await modal.getByTestId('field-username').fill(updated.username);
    await modal.getByTestId('field-email').fill(updated.mail);
    await modal.getByTestId('field-first_name').fill(updated.firstName);
    await modal.getByTestId('field-last_name').fill(updated.lastName);
    await modal.getByTestId('field-phone').fill(updated.phone);
    const responsePromise = page.waitForResponse(
      (resp) =>
        resp.url().endsWith(`/user/${testUser.username}`) &&
        resp.request().method() === 'PUT',
    );
    await modal.getByRole('button', { name: 'Save changes', exact: true }).click();
    expect((await responsePromise).status()).toBe(200);
    await modal.waitFor({ state: 'hidden' });

    await page.reload();
    await expect(
      page.locator('.virtual-row').filter({ hasText: updated.username }),
    ).toBeVisible();
    await expect(
      page.locator('.virtual-row').filter({ hasText: testUser.username }),
    ).toHaveCount(0);

    await page.goto(routes.base + routes.profile + updated.username + routes.tab.details);
    const card = page.locator('#general-card');
    await expect(card.getByTestId('field-username')).toHaveValue(updated.username);
    await expect(card.getByTestId('field-email')).toHaveValue(updated.mail);
    await expect(card.getByTestId('field-first_name')).toHaveValue(updated.firstName);
    await expect(card.getByTestId('field-last_name')).toHaveValue(updated.lastName);
    await expect(card.getByTestId('field-phone')).toHaveValue(updated.phone);
  });

  test('Add new device for enrolled user', async ({ page, browser, request }) => {
    const testUser = { ...testUserTemplate, username: 'testuserfirst' };
    const devicePubkey = 'DwcCqbwTEvI4erU8RrTUg3fRILhBVzy3rrTqEPGYKIA=';

    await waitForBase(page);
    const { token: activationToken } = await createUserEnrollment(browser, testUser);
    await apiEnrollmentStart(request, activationToken);
    await apiEnrollmentActivateUser(request, testUser.password, testUser.phone);

    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.identity.users);
    const userRow = page.locator('.virtual-row').filter({ hasText: testUser.username });
    await expect(userRow).toContainText('Enrolled');
    await userRow.locator('.icon-button').click();
    await page.locator('.menu').getByText('Add new device', { exact: true }).click();
    const modal = page.locator('#add-new-device-modal');
    await modal.waitFor({ state: 'visible' });
    await modal.getByTestId('add-new-device-manually').click();
    await modal.getByRole('button', { name: 'Submit', exact: true }).click();

    const deliveryStep = modal.locator('#add-new-device-delivery-step');
    await deliveryStep.waitFor({ state: 'visible' });
    const deviceToken = await deliveryStep
      .locator('.copy-field .track p')
      .nth(1)
      .textContent();
    expect(deviceToken).toBeTruthy();
    await deliveryStep.getByRole('button', { name: 'Close', exact: true }).click();

    await apiEnrollmentCreateDevice(request, deviceToken as string, {
      name: 'enrolled device',
      pubkey: devicePubkey,
    });

    await page.goto(
      routes.base + routes.profile + testUser.username + routes.tab.devices,
    );
    await expect(page.getByText('enrolled device', { exact: true })).toBeVisible();
  });

  test('Initiate self-enrollment for not enrolled user', async ({
    page,
    browser,
    request,
  }) => {
    const testUser = { ...testUserTemplate, username: 'pendinguser' };
    await setupSMTP(browser);
    const { token: firstToken } = await createUserEnrollment(browser, testUser);
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.identity.users);

    await page
      .locator('.virtual-row')
      .filter({ hasText: defaultUserAdmin.username })
      .locator('.icon-button')
      .click();
    await expect(page.locator('.menu')).toBeVisible();
    await expect(
      page.locator('.menu').getByText('Initiate self-enrollment', { exact: true }),
    ).toHaveCount(0);
    await page.keyboard.press('Escape');

    const row = page.locator('.virtual-row').filter({ hasText: testUser.username });
    await row.locator('.icon-button').click();
    await expect(
      page.locator('.menu').getByText('Add new device', { exact: true }),
    ).toHaveCount(0);
    await page
      .locator('.menu')
      .getByText('Initiate self-enrollment', { exact: true })
      .click();

    const modal = page.locator('#enrollment-token-modal');
    await expect(modal).toContainText('Initiate self-enrollment for user');
    await expect(modal).toContainText(
      'Provide the user with their token and URL in a convenient manner so they can ' +
        'complete the enrollment process independently.',
    );
    await expect(modal.getByTestId('activation-url-field')).toContainText(
      testsConfig.ENROLLMENT_URL,
    );
    const tokenField = modal.getByTestId('activation-token-field');
    await expect(tokenField).not.toBeEmpty();
    const token = (await tokenField.textContent()) ?? '';
    expect(token).not.toBe(firstToken);
    await expect(modal.getByRole('button', { name: 'Close' })).toBeVisible();
    await expect(modal.getByTestId('field-email')).toBeHidden();

    await apiClearMailbox(request);
    await modal
      .locator('.checkbox', { hasText: 'Send enrollment details to user by email' })
      .click();
    await expect(modal.getByTestId('field-email')).toHaveValue(testUser.mail);
    await expect(modal.getByRole('button', { name: 'Close' })).toHaveCount(0);
    await modal.getByRole('button', { name: 'Send email' }).click();
    await expect(page.getByText('Enrollment email sent successfully')).toBeVisible();
    await expect(modal).toBeHidden();

    const mail = await apiWaitForMail(request, testUser.mail);
    expect(mail.Subject).toBe('Defguard: User enrollment');
    expect(mail.Text).toContain(`URL: ${testsConfig.ENROLLMENT_URL}/`);
    const mailToken = mail.Text.split('Token: ')[1].split('\n')[0].trim();
    const enrollment = await apiEnrollmentStart(request, mailToken);
    expect(enrollment.user.login).toBe(testUser.username);
  });
});
