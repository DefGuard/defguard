import { expect, test } from '@playwright/test';

import { testsConfig, testUserTemplate } from '../config';
import { User } from '../types';
import { createUserEnrollment } from '../utils/controllers/enrollment';
import {
  setEdgeUiControls,
  waitForEdgeSettings,
} from '../utils/controllers/enrollmentSettings';
import { setupSMTP } from '../utils/controllers/settings';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

// These tests exercise the "control Edge UI by Core settings" feature (#3108):
// admins can hide the password-reset option and the client download step on
// Edge via enterprise settings. Both settings require an active business
// license - without one the toggles are disabled and Core ignores the stored
// values - so the suite is skipped when no license key is configured.
test.describe('Edge UI controls', () => {
  test.beforeEach(() => {
    test.skip(
      !process.env.DEFGUARD_LICENSE_KEY,
      'Edge UI controls require an active business license (DEFGUARD_LICENSE_KEY)',
    );
    dockerRestart();
  });

  test('Password reset is hidden when disabled in settings', async ({
    page,
    browser,
  }) => {
    await waitForBase(page);
    // Configure SMTP so password reset would be shown if the setting were on;
    // this proves the setting - not missing SMTP - is what hides it.
    await setupSMTP(browser);
    await setEdgeUiControls(browser, { displayPasswordReset: false });
    await waitForEdgeSettings(page, { displayPasswordReset: false });

    await page.goto(testsConfig.ENROLLMENT_URL);
    await expect(page.getByTestId('start-password-reset')).toBeHidden();
    await expect(page.locator('#home-choice')).toHaveClass(/single/);
  });

  test('Download step is skipped when disabled in settings', async ({
    page,
    browser,
  }) => {
    const user: User = { ...testUserTemplate, username: 'test' };
    await setEdgeUiControls(browser, { displayDownloadStep: false });
    await waitForEdgeSettings(page, { displayDownloadStep: false });
    const { token } = await createUserEnrollment(browser, user);

    // Starting enrollment with the download step disabled goes straight to
    // client setup instead of the download page.
    await page.goto(`${testsConfig.ENROLLMENT_URL}/?token=${token}`);
    await expect(page).toHaveURL(/\/client-setup$/);
  });

  test('Download step is shown by default', async ({ page, browser }) => {
    const user: User = { ...testUserTemplate, username: 'test' };
    await waitForEdgeSettings(page, { displayDownloadStep: true });
    const { token } = await createUserEnrollment(browser, user);

    await page.goto(`${testsConfig.ENROLLMENT_URL}/?token=${token}`);
    await expect(page).toHaveURL(/\/download$/);
  });
});
