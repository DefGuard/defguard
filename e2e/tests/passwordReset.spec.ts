import { expect, test } from '@playwright/test';

import { testsConfig, testUserTemplate } from '../config';
import { User } from '../types';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import {
  selectPasswordReset,
  setEmail,
  setPassword,
} from '../utils/controllers/passwordReset';
import { disableUser } from '../utils/controllers/toggleUserState';
import { getPasswordResetToken } from '../utils/db/getPasswordResetToken';
import { dockerDown, dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';

const newPassword = '!7(8o3aN8RoF';

test.describe('Reset password', () => {
  const user: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    await createUser(browser, user);
  });

  test.afterAll(() => {
    dockerDown();
  });

  test('Reset user password', async ({ page }) => {
    await waitForBase(page);
    await page.goto(testsConfig.ENROLLMENT_URL);
    await waitForPromise(2000);
    await selectPasswordReset(page);
    await setEmail(user.mail, page);

    const token = await getPasswordResetToken(user.mail);

    await page.goto(`${testsConfig.ENROLLMENT_URL}/password-reset/?token=${token}`);
    await waitForPromise(2000);

    await setPassword(newPassword, page);
    await page.getByTestId('password-reset-success').waitFor({ state: 'visible' });

    await waitForBase(page);
    await loginBasic(page, { ...user, password: newPassword });
    await logout(page);
  });

  test('Reset disabled user password', async ({ page, browser }) => {
    await waitForBase(page);
    await page.goto(testsConfig.ENROLLMENT_URL);
    await waitForPromise(2000);
    await selectPasswordReset(page);
    await setEmail(user.mail, page);
    await waitForPromise(2000);
    const token = await getPasswordResetToken(user.mail);
    await disableUser(browser, user);
    await page.goto(`${testsConfig.ENROLLMENT_URL}/password-reset/?token=${token}`);
    await waitForPromise(2000);

    // A message should be displayed that the code is invalid
    const message = await page.locator('.message').textContent();
    expect(message).toBe(
      'The entered code is invalid. Please start the process from the beginning.'
    );

    // The password input should not be visible
    const passwordInputVisible = await page
      .locator('[data-testid="field-password"]')
      .isVisible();
    expect(passwordInputVisible).toBe(false);
  });
});
