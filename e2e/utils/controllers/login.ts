import { Page } from 'playwright';
import { TOTP } from 'totp-generator';

import { routes, testsConfig } from '../../config';
import { User } from '../../types';
import { waitForRoute } from '../waitForRoute';

type AuthInfo = User | Pick<User, 'username' | 'password'>;

/**
 * Login with default admin pass given context is unauthenticated.
 */
export const loginBasic = async (page: Page, userInfo: AuthInfo) => {
  await page.goto(testsConfig.BASE_URL);
  await waitForRoute(page, routes.auth.login);
  await page.getByTestId('field-username').fill(userInfo.username);
  await page.getByTestId('field-password').fill(userInfo.password);
  // Set up the handler immediately before click to avoid matching pre-existing responses.
  // Explicitly check POST to avoid matching any GET auth-check requests the SPA may fire.
  const responsePromise = page.waitForResponse(
    (resp) => resp.url().endsWith('/api/v1/auth') && resp.request().method() === 'POST',
  );
  await page.getByTestId('sign-in').click();
  await responsePromise;
};

export const loginTOTP = async (page: Page, userInfo: AuthInfo, totpSecret: string) => {
  await page.goto(testsConfig.BASE_URL);
  await waitForRoute(page, routes.auth.login);
  await page.getByTestId('field-username').fill(userInfo.username);
  await page.getByTestId('field-password').fill(userInfo.password);
  await page.getByTestId('sign-in').click();
  await waitForRoute(page, routes.auth.totp);
  const codeField = await page.getByTestId('field-code');
  await codeField.clear();
  const { otp: token } = TOTP.generate(totpSecret);
  await codeField.fill(token);
  const responsePromise = page.waitForResponse(
    (resp) => resp.url().includes('/api/v1/auth') && resp.request().method() === 'POST',
  );
  await page.getByTestId('submit-totp').click();
  await responsePromise;
};

export const loginRecoveryCodes = async (
  page: Page,
  userInfo: AuthInfo,
  code: string,
): Promise<void> => {
  await page.goto(testsConfig.BASE_URL);
  await waitForRoute(page, routes.auth.login);
  await page.getByTestId('field-username').fill(userInfo.username);
  await page.getByTestId('field-password').fill(userInfo.password);
  await page.getByTestId('sign-in').click();
  await page.locator('a:has-text("Use recovery codes instead")').click();
  await page.getByTestId('field-code').waitFor({ state: 'visible' });
  await page.getByTestId('field-code').clear();
  await page.getByTestId('field-code').fill(code.trim());
  const responsePromise = page.waitForResponse(
    (resp) => resp.url().includes('/api/v1/auth') && resp.request().method() === 'POST',
  );
  await page.getByTestId('submit-recovery-code').click();
  await responsePromise;
};
