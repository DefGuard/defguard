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
    await expect(row).toContainText('Pending');

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

test.describe('Enrollment on Edge page', () => {
  const user: User = { ...testUserTemplate, username: 'edgeuser' };

  test.beforeEach(() => dockerRestart());

  test('User goes through enrollment on Edge page to client setup', async ({
    page,
    browser,
  }) => {
    const { token } = await createUserEnrollment(browser, user);

    await page.goto(testsConfig.ENROLLMENT_URL);
    await page.getByTestId('start-enrollment').click();
    await expect(page.getByText('Step 1/3')).toBeVisible();
    await page.getByTestId('field-token').fill(token);
    await page.getByTestId('page-nav-next').click();

    await page.waitForURL('**/download');
    await expect(page.getByText('Step 2/3')).toBeVisible();
    for (const system of ['windows', 'linux', 'macos']) {
      await expect(page.getByTestId(system)).toBeVisible();
    }
    await page.getByTestId('page-nav-next').click();
    await expect(page.getByText('Download confirmation')).toBeVisible();
    await page.getByTestId('modal-confirm-download-submit').click();

    await page.waitForURL('**/client-setup');
    await expect(page.getByText('Configure your Defguard client')).toBeVisible();
    await expect(
      page.getByRole('link', { name: 'One-Click Configuration' }),
    ).toHaveAttribute(
      'href',
      `defguard://addinstance?token=${token}&url=${testsConfig.ENROLLMENT_URL}/`,
    );
    await page.getByText('Show advanced configuration').click();
    await expect(
      page.getByText(`${testsConfig.ENROLLMENT_URL}/`, { exact: true }),
    ).toBeVisible();
    await expect(page.getByText(token, { exact: true })).toBeVisible();
    await expect(page.getByRole('link', { name: defaultUserAdmin.mail })).toBeVisible();

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const startedRow = page
      .locator('#activity-log-page .virtual-row')
      .filter({ hasText: 'Enrollment started' });
    await expect(async () => {
      await page.goto(routes.base + routes.activity);
      await expect(startedRow).toBeVisible({ timeout: 1000 });
    }).toPass();
    await expect(startedRow).toContainText(user.username);
  });

  test('Invalid token is rejected on Edge page', async ({ page }) => {
    await page.goto(testsConfig.ENROLLMENT_URL);
    await page.getByTestId('start-enrollment').click();
    await page.getByTestId('field-token').fill('invalid-token');
    await page.getByTestId('page-nav-next').click();

    await expect(page.getByText('Token is not valid')).toBeVisible();
    await expect(page.getByText('Step 1/3')).toBeVisible();
    await expect(page.getByTestId('field-token')).toHaveValue('invalid-token');
  });
});
