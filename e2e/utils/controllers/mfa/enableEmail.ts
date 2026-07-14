import { Browser } from 'playwright';
import { TOTP } from 'totp-generator';

import { routes } from '../../../config';
import { User } from '../../../types';
import { extractEmailSecret } from '../../db/extractEmailSecret';
import { waitForBase } from '../../waitForBase';
import { acceptRecovery } from '../acceptRecovery';
import { loginBasic } from '../login';
import { setupSMTP } from '../settings';

export type EnableEmailResult = {
  secret: string;
  recoveryCodes?: string[];
};

export const enableEmailMFA = async (
  browser: Browser,
  user: User,
): Promise<EnableEmailResult> => {
  await setupSMTP(browser);
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, user);
  await page.goto(routes.base + routes.profile + user.username);
  await page.getByTestId('email-codes-row').locator('.icon-button').click();
  await page.getByTestId('enable-email').click();
  await page.getByTestId('field-code').waitFor({ state: 'visible' });
  const secret = await extractEmailSecret(user.username);
  const { otp: code } = await TOTP.generate(secret, {
    digits: 6,
    period: 300,
  });
  await page.getByTestId('field-code').fill(code);
  await page.getByTestId('submit').click();
  const recovery = await acceptRecovery(page);
  await context.close();
  return {
    secret,
    recoveryCodes: recovery,
  };
};
