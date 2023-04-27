import { expect } from '@playwright/test';
import { Page } from 'playwright';

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
  console.log(response.status());
  expect([200, 201].includes(response.status())).toBeTruthy();
  await waitForPromise(2000);
};

export const loginTOTP = async (page: Page, userInfo: AuthInfo, totpToken: string) => {
  await loginBasic(page, userInfo);
  await waitForRoute(page, routes.auth.totp);
  await page.getByTestId('field-code').type(totpToken);
  await page.locator('button[type="submit"]').click();
};
