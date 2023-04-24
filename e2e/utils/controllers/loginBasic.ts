import { Page } from 'playwright';

import { routes, testsConfig } from '../../config';
import { User } from '../../types';

/**
 * Login with default admin pass given context is unauthenticated.
 */
export const loginBasic = async (
  page: Page,
  userInfo: User | Pick<User, 'username' | 'password'>
) => {
  await page.goto(testsConfig.BASE_URL);
  await page.waitForURL(routes.auth.login, {
    waitUntil: 'networkidle',
  });
  await page.getByTestId('login-form-username').type(userInfo.username);
  await page.getByTestId('login-form-password').type(userInfo.password);
  await page.getByTestId('login-form-submit').click();
  await page.waitForLoadState('networkidle');
};
