import { Browser, expect, Page } from '@playwright/test';

import { defaultUserAdmin, routes, testsConfig } from '../../config';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';
import { logout } from './logout';

type EdgeUiControls = {
  displayPasswordReset?: boolean;
  displayDownloadStep?: boolean;
};

// Toggle the Edge UI controls (enterprise settings) via the admin Enrollment
// settings page. After a `dockerRestart()` both settings are at their `true`
// defaults, so a value of `false` means "click the toggle once to turn it off"
// and `true` means "leave it on". Requires an active business license, without
// which the toggles are disabled and Core ignores the stored values.
export const setEdgeUiControls = async (browser: Browser, controls: EdgeUiControls) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  // loginBasic resolves as soon as the auth request returns, before the SPA has
  // finished its post-login redirect. Navigating too early makes the authorized
  // route guard bounce back to the login page, so wait until we've left it.
  await page.waitForURL((url) => !url.pathname.startsWith('/auth'));
  await page.goto(routes.base + routes.enrollment);

  if (controls.displayPasswordReset === false) {
    await page.getByTestId('field-display-password-reset').click();
  }
  if (controls.displayDownloadStep === false) {
    await page.getByTestId('field-display-download-step').click();
  }

  const saveButton = page.getByTestId('save-enrollment-settings');
  await expect(saveButton).toBeEnabled();
  await saveButton.click();
  // On a successful save the form is reset to the stored values, so the button
  // returns to its disabled (pristine) state.
  await expect(saveButton).toBeDisabled();

  await logout(page);
  await context.close();
};

// Poll the proxy's public info endpoint until the Edge UI controls it reports
// match the expected values. Core pushes settings changes to the proxy
// asynchronously, so this closes the propagation race before assertions.
export const waitForEdgeSettings = async (page: Page, expected: EdgeUiControls) => {
  await expect
    .poll(
      async () => {
        const res = await page.request.get(`${testsConfig.ENROLLMENT_URL}/api/v1/info`);
        if (!res.ok()) return false;
        const body = await res.json();
        if (
          expected.displayPasswordReset !== undefined &&
          body.display_password_reset !== expected.displayPasswordReset
        ) {
          return false;
        }
        if (
          expected.displayDownloadStep !== undefined &&
          body.display_download_step !== expected.displayDownloadStep
        ) {
          return false;
        }
        return true;
      },
      { timeout: 30_000 },
    )
    .toBe(true);
};
