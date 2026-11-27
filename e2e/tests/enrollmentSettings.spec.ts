import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { User } from '../types';
import { apiEnrollmentStart } from '../utils/api/enrollment';
import { createUserEnrollment } from '../utils/controllers/enrollment';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

test.describe('Enrollment settings', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(() => dockerRestart());

  test('Welcome message is saved and returned at enrollment start', async ({
    page,
    browser,
    request,
  }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.enrollment);
    await page.locator('.tab').filter({ hasText: 'Message templates' }).click();

    const messageBlock = page
      .locator('.interactive-block')
      .filter({ hasText: 'Display welcome message' });
    await messageBlock.locator('textarea').fill('Welcome {{ first_name }} on board!');
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page.getByText('Settings saved')).toBeVisible();

    await page.reload();
    await page.locator('.tab').filter({ hasText: 'Message templates' }).click();
    await expect(messageBlock.locator('textarea')).toHaveValue(
      'Welcome {{ first_name }} on board!',
    );

    const { token } = await createUserEnrollment(browser, testUser);
    const start = await apiEnrollmentStart(request, token);
    expect(start.final_page_content).toContain(`Welcome ${testUser.firstName} on board!`);
  });

  test('Hidden welcome message is not returned at enrollment start', async ({
    page,
    browser,
    request,
  }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.enrollment);
    await page.locator('.tab').filter({ hasText: 'Message templates' }).click();

    await page
      .locator('.interactive-block')
      .filter({ hasText: 'Display welcome message' })
      .locator('.interactive')
      .first()
      .click();
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page.getByText('Settings saved')).toBeVisible();

    const { token } = await createUserEnrollment(browser, testUser);
    const start = await apiEnrollmentStart(request, token);
    expect(start.user.login).toBe(testUser.username);
    expect(start.final_page_content ?? '').toBe('');
  });

  test('Welcome email settings are saved', async ({ page }) => {
    const subject = 'Hello from e2e';
    const body = 'Custom welcome email body';
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.enrollment);
    await page.locator('.tab').filter({ hasText: 'Message templates' }).click();

    const emailBlock = page
      .locator('.interactive-block')
      .filter({ hasText: 'Send welcome email' });
    await page.getByTestId('field-enrollment_welcome_email_subject').fill(subject);
    const sameAsMessage = page.getByTestId(
      'field-enrollment_use_welcome_message_as_email',
    );
    await expect(sameAsMessage).toHaveAttribute('data-active', 'true');
    await sameAsMessage.click();
    await emailBlock.getByRole('textbox', { name: 'Email message' }).fill(body);
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page.getByText('Settings saved')).toBeVisible();

    await page.reload();
    await page.locator('.tab').filter({ hasText: 'Message templates' }).click();
    await expect(page.getByTestId('field-enrollment_welcome_email_subject')).toHaveValue(
      subject,
    );
    await expect(sameAsMessage).toHaveAttribute('data-active', 'false');
    await expect(emailBlock.getByRole('textbox', { name: 'Email message' })).toHaveValue(
      body,
    );
  });

  test('Enrollment session duration is saved', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.enrollment);

    const sessionSelect = page.getByTestId('field-enrollment_session_timeout_minutes');
    await sessionSelect.click();
    await page
      .locator('.select-floating .select-option')
      .filter({ hasText: '30 minutes' })
      .click();
    await page.getByTestId('save-enrollment-settings').click();
    await expect(page.getByText('Settings saved')).toBeVisible();

    await page.reload();
    await expect(sessionSelect).toContainText('30 minutes');
  });
});
