import { expect } from '@playwright/test';
import { Page } from 'playwright';
import { TOTP } from 'totp-generator';

import { routes, testsConfig } from '../../config';
import { User } from '../../types';
import { waitForPromise } from '../waitForPromise';
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
  await page.getByTestId('sign-in').click();
  await waitForPromise(1000);
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
  await page.getByTestId('submit-totp').click();
  await waitForPromise(1000);
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
  await waitForPromise(1000);
  await page.getByTestId('field-code').clear();
  await page.getByTestId('field-code').fill(code.trim());
  await page.getByTestId('submit-recovery-code').click();
  await waitForPromise(1000);
};
