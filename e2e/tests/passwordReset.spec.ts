import { expect, test } from '@playwright/test';

import { testsConfig, testUserTemplate } from '../config';
import { User } from '../types';
import { createUser } from '../utils/controllers/createUser';
import enrollmentController from '../utils/controllers/enrollment';
import { loginBasic } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { getPasswordResetToken } from '../utils/db/getPasswordResetToken';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const newPassword = '!7(8o3aN8RoF';

test.describe('Reset password', () => {
  const user: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    await createUser(browser, user);
  });

  test('Reset user password', async ({ page }) => {
    await waitForBase(page);
    await page.goto(testsConfig.ENROLLMENT_URL, {
      waitUntil: 'networkidle',
    });
    await enrollmentController.startPasswordReset(page);
    await enrollmentController.fillPasswordResetStartForm(user.mail, page);
    const response = page.waitForResponse((resp) => resp.url().endsWith('/request'));
    await enrollmentController.navNext(page);
    expect((await response).ok()).toBeTruthy();
    const token = await getPasswordResetToken(user.mail);

    await page.goto(`${testsConfig.ENROLLMENT_URL}/password-reset/?token=${token}`);

    await enrollmentController.fillPasswordResetForm(
      {
        password: newPassword,
        repeat: newPassword,
      },
      page,
    );

    const resetPromise = page.waitForResponse((response) =>
      response.url().endsWith('/finish'),
    );
    await page.getByTestId('form-submit').click();

    expect((await resetPromise).ok()).toBeTruthy();

    await page.waitForURL('**/password/finish', {
      waitUntil: 'networkidle',
    });

    // check if can be logged in with new password
    await waitForBase(page);
    await loginBasic(page, { ...user, password: newPassword });
    await logout(page);
  });
});
