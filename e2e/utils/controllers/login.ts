import { Page } from 'playwright';

import { routes, testsConfig } from '../../config';
import { User } from '../../types';

type AuthInfo = User | Pick<User, 'username' | 'password'>;

/**
 * Login with default admin pass given context is unauthenticated.
 */
export const loginBasic = async (page: Page, userInfo: AuthInfo) => {
  await page.goto(testsConfig.BASE_URL);
  await page.waitForURL(routes.auth.login, {
    waitUntil: 'networkidle',
  });
  await page.getByTestId('login-form-username').type(userInfo.username);
  await page.getByTestId('login-form-password').type(userInfo.password);
  await page.getByTestId('login-form-submit').click();
};

export const loginTOTP = async (page: Page, userInfo: AuthInfo, totpToken: string) => {
  await loginBasic(page, userInfo);
  await page.waitForURL(routes.base + routes.auth.totp);
  await page.getByTestId('field-code').type(totpToken);
  await page.locator('button[type="submit"]').click();
};
