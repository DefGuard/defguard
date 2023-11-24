import { Page } from 'playwright';
import totp from 'totp-generator';

import { routes } from '../../../config';
import { getPageClipboard } from '../../getPageClipboard';
import { waitForRoute } from '../../waitForRoute';

export const enableTOTP = async (page: Page): Promise<string> => {
  await page.goto(routes.base + routes.me);
  await waitForRoute(page, routes.me);
  await page.getByTestId('edit-user').click();
  await page.getByTestId('edit-totp').click();
  await page.getByTestId('enable-totp-option').click();
  await page.getByTestId('copy-totp').click();
  const totpSecret = await getPageClipboard(page);
  const token = totp(totpSecret);
  const totpForm = page.getByTestId('register-totp-form');
  await totpForm.getByTestId('field-code').type(token);
  await totpForm.locator('button[type="submit"]').click();
  await totpForm.waitFor({ state: 'hidden' });
  return totpSecret;
};
