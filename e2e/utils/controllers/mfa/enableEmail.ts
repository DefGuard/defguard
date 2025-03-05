import { Browser } from 'playwright';
import { TOTP } from 'totp-generator';

import { defaultUserAdmin, routes } from '../../../config';
import { User } from '../../../types';
import { extractEmailSecret } from '../../db/extractEmailSecret';
import { waitForBase } from '../../waitForBase';
import { waitForPromise } from '../../waitForPromise';
import { acceptRecovery } from '../acceptRecovery';
import { loginBasic } from '../login';
import { logout } from '../logout';

export type EnableEmailResult = {
  secret: string;
  recoveryCodes?: string[];
};

export const setupSMTP = async (browser: Browser) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await waitForPromise(5000);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.admin.settings);
  await page.getByRole('button', { name: 'SMTP' }).click();
  await page.getByTestId('field-smtp_server').fill('testServer.com');
  await page.getByTestId('field-smtp_port').fill('543');
  await page.getByTestId('field-smtp_user').fill('testuser');
  await page.getByTestId('field-smtp_password').fill('test');
  await page.getByTestId('field-smtp_sender').fill('test@test.com');
  const requestPromise = page.waitForRequest('**/settings');
  await page.getByRole('button', { name: 'Save changes' }).click();
  await requestPromise;
  await waitForPromise(1000);
  await logout(page);
  await context.close();
};

export const enableEmailMFA = async (
  browser: Browser,
  user: User
): Promise<EnableEmailResult> => {
  await setupSMTP(browser);
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  // make so app info will allow email mfa to be enabled for user
  await waitForPromise(5000);
  await loginBasic(page, user);
  await page.goto(routes.base + routes.me);
  await page.getByTestId('edit-user').click();
  await page.getByTestId('edit-email-mfa').click();
  const requestPromise = page.waitForRequest('**/init');
  await page.getByTestId('enable-email-mfa-option').click();
  const formElement = page.locator('#register-mfa-email-form');
  await requestPromise;
  await waitForPromise(2000);
  const secret = await extractEmailSecret(user.username);
  const {otp: code } = TOTP.generate(secret, {
    digits: 6,
    period: 60,
  });
  await page.getByTestId('field-code').type(code);
  await formElement.locator('button[type="submit"]').click();
  await formElement.waitFor({ state: 'detached', timeout: 1000 });
  const recovery = await acceptRecovery(page);
  await context.close();
  return {
    secret,
    recoveryCodes: recovery,
  };
};
