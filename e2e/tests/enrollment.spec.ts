import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testsConfig, testUserTemplate } from '../config';
import { NetworkForm, User } from '../types';
import { apiEnrollmentActivateUser, apiEnrollmentStart } from '../utils/api/enrollment';
import { apiClearMailbox, apiWaitForMail } from '../utils/api/mailpit';
import { createUserEnrollment } from '../utils/controllers/enrollment';
import { loginBasic } from '../utils/controllers/login';
import { setupSMTP } from '../utils/controllers/settings';
import { disableUser } from '../utils/controllers/toggleUserState';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const testNetwork: NetworkForm = {
  name: 'test network',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  allowed_ips: ['127.1.5.1'],
  port: '5055',
};

test.describe('Create user and enroll him', () => {
  let token: string;
  const user: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    const response = await createUserEnrollment(browser, user);
    token = response.token;
    await createRegularLocation(browser, testNetwork);
  });

  test('Complete user enrollment via API', async ({ request, page }) => {
    expect(token).toBeDefined();
    await apiEnrollmentStart(request, token);
    await apiEnrollmentActivateUser(request, user.password, '+48123456789');

    await waitForBase(page);
    const responsePromise = page.waitForResponse('**/auth');
    await loginBasic(page, { username: user.username, password: user.password });
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
  });
  test('Try to complete disabled user enrollment via API', async ({
    page,
    request,
    browser,
  }) => {
    expect(token).toBeDefined();
    await disableUser(browser, user);
    await apiEnrollmentStart(request, token);
    await apiEnrollmentActivateUser(request, user.password, '+48123456789');

    await waitForBase(page);
    const responsePromise = page.waitForResponse('**/auth');
    await loginBasic(page, { username: user.username, password: user.password });
    const response = await responsePromise;
    expect(response.ok()).toBeFalsy();
  });
});

test.describe('Add user with self-enrollment', () => {
  test.beforeEach(() => dockerRestart());

  test('User is pending, gets enrollment email and token is logged', async ({
    page,
    browser,
    request,
  }) => {
    const user: User = { ...testUserTemplate, username: 'selfenrolled' };
    await setupSMTP(browser);
    await apiClearMailbox(request);
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);

    await page.goto(routes.base + routes.identity.users);
    await page.getByTestId('add-user').click();
    await page.getByTestId('add-user-self-enrollment').click();
    const modal = page.locator('#add-user-modal');
    await modal.getByTestId('field-username').fill(user.username);
    await modal.getByTestId('field-first_name').fill(user.firstName);
    await modal.getByTestId('field-last_name').fill(user.lastName);
    await modal.getByTestId('field-email').fill(user.mail);
    await modal.getByTestId('field-phone').fill(user.phone);
    await modal.getByTestId('add-user-submit').click();

    await expect(modal).toContainText('Start enrollment for user');
    await expect(modal.getByTestId('activation-token-field')).not.toBeEmpty();
    await modal
      .locator('.checkbox', { hasText: 'Send enrollment details to user by email' })
      .click();
    await expect(modal.getByTestId('field-email')).toHaveValue(user.mail);
    await modal.getByRole('button', { name: 'Send email' }).click();
    await expect(page.getByText('Enrollment email sent successfully')).toBeVisible();

    const mail = await apiWaitForMail(request, user.mail);
    expect(mail.Subject).toBe('Defguard: User enrollment');
    expect(mail.Text).toContain(`URL: ${testsConfig.ENROLLMENT_URL}/`);
    const mailToken = mail.Text.split('Token: ')[1].split('\n')[0].trim();
    const enrollment = await apiEnrollmentStart(request, mailToken);
    expect(enrollment.user.login).toBe(user.username);

    await page.reload();
    const row = page.locator('.virtual-row').filter({ hasText: user.username });
    await expect(row.locator('.table-cell').nth(8)).toHaveText('Pending');

    const tokenRow = page
      .locator('#activity-log-page .virtual-row')
      .filter({ hasText: 'Enrollment token added' });
    await expect(async () => {
      await page.goto(routes.base + routes.activity);
      await expect(tokenRow).toHaveCount(2, { timeout: 1000 });
    }).toPass();
    await expect(tokenRow.filter({ hasText: user.username })).toHaveCount(2);
  });
});
