import { Browser } from 'playwright';
import { TOTP } from 'totp-generator';

import { routes } from '../../../config';
import { User } from '../../../types';
import { extractEmailSecret } from '../../db/extractEmailSecret';
import { waitForBase } from '../../waitForBase';
import { acceptRecovery } from '../acceptRecovery';
import { loginBasic, loginTOTP } from '../login';
import { setupSMTP } from '../settings';

export type EnableEmailResult = {
  secret: string;
  recoveryCodes?: string[];
};

export const enableEmailMFA = async (
  browser: Browser,
  user: User,
  totpSecret?: string,
): Promise<EnableEmailResult> => {
  await setupSMTP(browser);
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  if (totpSecret) {
    await loginTOTP(page, user, totpSecret);
  } else {
    await loginBasic(page, user);
  }
  await page.goto(routes.base + routes.profile + user.username);
  await page.getByTestId('email-codes-row').locator('.icon-button').click();
  const initResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().endsWith('/api/v1/auth/email/init'),
  );
  await page.getByTestId('enable-email').click();
  const response = await initResponse;
  if (response.status() !== 200) {
    throw new Error(`Email MFA initialization failed: ${response.status()}`);
  }
  await page.getByTestId('field-code').waitFor({ state: 'visible' });
  const secret = await extractEmailSecret(user.username);
  const { otp: code } = await TOTP.generate(secret, {
    digits: 6,
    period: 300,
  });
  await page.getByTestId('field-code').fill(code);
  await page.getByTestId('submit').click();
  const recovery = totpSecret ? undefined : await acceptRecovery(page);
  if (totpSecret) {
    await page
      .getByTestId('email-codes-row')
      .getByText('Enabled', { exact: true })
      .waitFor({ state: 'visible' });
  }
  await context.close();
  return {
    secret,
    recoveryCodes: recovery,
  };
};
