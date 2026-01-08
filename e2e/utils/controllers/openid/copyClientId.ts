import { Browser } from 'playwright';

import { defaultUserAdmin, routes } from '../../../config';
import { getPageClipboard } from '../../getPageClipboard';
import { waitForBase } from '../../waitForBase';
import { loginBasic } from '../login';

export const copyOpenIdClientId = async (browser: Browser, clientId: number) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.openid_apps, { waitUntil: 'networkidle' });
  const deviceRow = page.locator('.virtual-row').nth(clientId - 1);
  await deviceRow.locator('.icon-button').click();
  await page.getByTestId('copy-id').click();
  const id = await getPageClipboard(page);
  return id;
};

export const copyOpenIdClientIdAndSecret = async (
  browser: Browser,
  clientName: string,
) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.openid_apps, { waitUntil: 'networkidle' });
  const userRow = await page.locator('.virtual-row').filter({ hasText: clientName });
  await userRow.locator('.icon-button').click();
  await page.getByTestId('copy-id').click();
  const id = await getPageClipboard(page);

  await userRow.locator('.icon-button').click();
  await page.getByTestId('copy-secret').click();

  const secret = await getPageClipboard(page);

  await context.close();
  return [id, secret];
};
