import { expect } from '@playwright/test';
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
  const totpURL = await getPageClipboard(page);
  expect(totpURL).toBeDefined();
  const secret = totpURL.split('secret=')[1];
  expect(secret.length).toBeGreaterThan(0);
  const token = totp(secret);
  const totpForm = page.getByTestId('register-totp-form');
  await totpForm.getByTestId('field-code').type(token);
  await totpForm.locator('button[type="submit"]').click();
  await totpForm.waitFor({ state: 'hidden' });
  return secret;
};
