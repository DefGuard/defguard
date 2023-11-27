import { Browser } from 'playwright';
import totp from 'totp-generator';

import { routes } from '../../../config';
import { User } from '../../../types';
import { extractEmailSecret } from '../../db/extractEmailSecret';
import { initSmtpSettings } from '../../db/initSmtpSettings';
import { waitForBase } from '../../waitForBase';
import { waitForPromise } from '../../waitForPromise';
import { acceptRecovery } from '../acceptRecovery';
import { loginBasic } from '../login';

export type EnableEmailResult = {
  secret: string;
  recoveryCodes?: string[];
};

export const enableEmailMFA = async (
  browser: Browser,
  user: User
): Promise<EnableEmailResult> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  // make so app info will allow email mfa to be enabled for user
  await waitForPromise(5000);
  await initSmtpSettings();
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
  const code = totp(secret);
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
