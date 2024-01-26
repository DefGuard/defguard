import { test } from '@playwright/test';

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
import { getPasswordResetToken } from '../utils/db/getPasswordResetToken';
import { dockerDown, dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';

const newPassword = '!7(8o3aN8RoF';

test.describe('Reset password', () => {
  const user: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser, page }) => {
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

    await page.getByTestId('email-sent-message').waitFor({ state: 'visible' });

    const token = await getPasswordResetToken(user.mail);

    await page.goto(`${testsConfig.ENROLLMENT_URL}/password-reset/?token=${token}`);
    await waitForPromise(2000);

    await setPassword(newPassword, page);
    await page.getByTestId('password-reset-success').waitFor({ state: 'visible' });

    await waitForBase(page);
    await loginBasic(page, { ...user, password: newPassword });
    await logout(page);
  });
});
