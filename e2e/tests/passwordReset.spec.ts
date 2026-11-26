import { expect, test } from '@playwright/test';

import { testsConfig, testUserTemplate } from '../config';
import { User } from '../types';
import { apiClearMailbox, apiWaitForMail } from '../utils/api/mailpit';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import {
  getPasswordResetLink,
  selectPasswordReset,
  setEmail,
  setPassword,
  waitForEdgePasswordResetEnabled,
} from '../utils/controllers/passwordReset';
import { setupSMTP } from '../utils/controllers/settings';
import { disableUser } from '../utils/controllers/toggleUserState';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const newPassword = '!7(8o3aN8RoF';

test.describe('Reset password', () => {
  const user: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    await createUser(browser, user);
  });

  test('Reset user password', async ({ page, browser, request }) => {
    await waitForBase(page);
    await setupSMTP(browser);
    await apiClearMailbox(request);
    await waitForEdgePasswordResetEnabled(page);
    await page.goto(testsConfig.ENROLLMENT_URL);
    await selectPasswordReset(page);
    await setEmail(user.mail, page);
    const resetLink = await getPasswordResetLink(request, user.mail);

    await page.goto(resetLink);
    await page.getByTestId('field-password').waitFor({ state: 'visible' });

    await setPassword(newPassword, page);
    const goToLogin = page.locator('button[data-variant="primary"]');
    await expect(goToLogin).toBeVisible();
    await goToLogin.click();

    const doneMail = await apiWaitForMail(
      request,
      user.mail,
      'Defguard: Password reset success',
    );
    expect(doneMail.Subject).toBe('Defguard: Password reset success');

    await waitForBase(page);
    await loginBasic(page, { ...user, password: newPassword });
    await logout(page);
  });

  test('Reset disabled user password', async ({ page, browser, request }) => {
    await waitForBase(page);
    await setupSMTP(browser);
    await apiClearMailbox(request);
    await waitForEdgePasswordResetEnabled(page);
    await page.goto(testsConfig.ENROLLMENT_URL);
    await selectPasswordReset(page);
    await setEmail(user.mail, page);
    const resetLink = await getPasswordResetLink(request, user.mail);
    await disableUser(browser, user);
    await page.goto(resetLink);

    await expect(
      page.locator('h1', { hasText: 'Link expired or invalid.' }),
    ).toBeVisible();
    await expect(page.locator('button[data-variant="primary"]')).toBeVisible();

    await expect(page.getByTestId('field-password')).toBeHidden();
  });
});
