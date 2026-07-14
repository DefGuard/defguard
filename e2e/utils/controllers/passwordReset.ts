import { expect } from '@playwright/test';
import { Page } from 'playwright';

import { testsConfig } from '../../config';

// The Edge home page only shows the password-reset option once Core reports
// display_password_reset === true (which requires SMTP to be configured and the
// setting enabled). Core pushes that value to the proxy asynchronously, so poll
// the proxy's public info endpoint until it is reflected before navigating.
export const waitForEdgePasswordResetEnabled = async (page: Page) => {
  await expect
    .poll(
      async () => {
        const res = await page.request.get(`${testsConfig.ENROLLMENT_URL}/api/v1/info`);
        if (!res.ok()) return false;
        const body = await res.json();
        return body.display_password_reset === true;
      },
      { timeout: 30_000 },
    )
    .toBe(true);
};

export const selectPasswordReset = async (page: Page) => {
  const selectButton = page.getByTestId('start-password-reset');
  await selectButton.waitFor({ state: 'visible' });
  await selectButton.click();
};

export const setEmail = async (email: string, page: Page) => {
  await page.getByTestId('field-email').waitFor({ state: 'visible' });
  await page.getByTestId('field-email').fill(email);
  await page.getByTestId('page-nav-next').click();
  // Wait for the email step to complete (field hidden = server processed the request).
  await page.getByTestId('field-email').waitFor({ state: 'hidden' });
};

export const setPassword = async (password: string, page: Page) => {
  await page.getByTestId('field-password').fill(password);
  await page.getByTestId('field-repeat').fill(password);
  await page.getByTestId('form-submit').click();
};
