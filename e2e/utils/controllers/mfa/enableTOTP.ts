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
  await page.goto(routes.base + routes.profile + user.username + '?tab=details');
  await waitForRoute(page, routes.profile + user.username + '?tab=details');
  const totpContainer = await page.locator('[data-testid="totp-row"]');

  await totpContainer.locator('.icon-button').click();
  await page.getByTestId('enable-totp').click();
  const totpSecret =
    (await page.getByTestId('totp-code').locator('p').textContent()) ?? '';

  const { otp: token } = TOTP.generate(totpSecret);

  await page.getByTestId('field-code').fill(token);
  await page.getByTestId('submit-totp').click();
  await page.getByTestId('copy-recovery-codes').click();
  const recoveryString = await getPageClipboard(page);
  const recovery = recoveryString.split('\n').filter((line) => line.trim());

  await page.getByTestId('confirm-code-save').click();
  await page.getByTestId('finish-recovery-codes').click();

  await context.close();
  return {
    secret: totpSecret,
    recoveryCodes: recovery,
  };
};
