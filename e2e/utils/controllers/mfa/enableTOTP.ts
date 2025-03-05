import { Browser } from 'playwright';
import { TOTP } from 'totp-generator';

import { routes } from '../../../config';
import { User } from '../../../types';
import { getPageClipboard } from '../../getPageClipboard';
import { waitForBase } from '../../waitForBase';
import { waitForRoute } from '../../waitForRoute';
import { acceptRecovery } from '../acceptRecovery';
import { loginBasic } from '../login';

export type EnableTOTPResult = {
  secret: string;
  recoveryCodes?: string[];
};

export const enableTOTP = async (
  browser: Browser,
  user: User,
): Promise<EnableTOTPResult> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, user);
  await page.goto(routes.base + routes.me);
  await waitForRoute(page, routes.me);
  await page.getByTestId('edit-user').click();
  await page.getByTestId('edit-totp').click();
  await page.getByTestId('enable-totp-option').click();
  await page.getByTestId('copy-totp').click();
  const totpSecret = await getPageClipboard(page);
  const { otp: token } = TOTP.generate(totpSecret);
  const totpForm = page.getByTestId('register-totp-form');
  await totpForm.getByTestId('field-code').type(token);
  await totpForm.locator('button[type="submit"]').click();
  await totpForm.waitFor({ state: 'hidden' });
  const recovery = await acceptRecovery(page);
  await context.close();
  return {
    secret: totpSecret,
    recoveryCodes: recovery,
  };
};
