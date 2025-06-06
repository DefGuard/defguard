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
  await page.getByTestId('login-form-username').type(userInfo.username);
  await page.getByTestId('login-form-password').type(userInfo.password);
  const responsePromise = page.waitForResponse('**/auth');
  await page.getByTestId('login-form-submit').click();
  const response = await responsePromise;
  expect([200, 201].includes(response.status())).toBeTruthy();
  await waitForPromise(2000);
};

export const loginTOTP = async (page: Page, userInfo: AuthInfo, totpSecret: string) => {
  await loginBasic(page, userInfo);
  await waitForRoute(page, routes.auth.totp);
  const codeField = page.getByTestId('field-code');
  await codeField.clear();
  const responsePromise = page.waitForResponse('**/verify');
  const { otp: token } = TOTP.generate(totpSecret);
  await codeField.type(token);
  await page.locator('button[type="submit"]').click();
  const response = await responsePromise;
  expect(response.status()).toBe(200);
};

export const loginRecoveryCodes = async (
  page: Page,
  userInfo: AuthInfo,
  code: string,
): Promise<void> => {
  await loginBasic(page, userInfo);
  await page.goto(routes.base + routes.auth.recovery, {
    waitUntil: 'networkidle',
  });
  await page.getByTestId('field-code').clear();
  await page.getByTestId('field-code').type(code.trim(), { delay: 100 });
  await page.locator('button[type="submit"]').click();
};
