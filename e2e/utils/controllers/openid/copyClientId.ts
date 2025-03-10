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
  await page.goto(routes.base + routes.admin.openid, { waitUntil: 'networkidle' });
  await page.getByTestId(`edit-openid-client-${clientId}`).click();
  await page.getByTestId('copy-openid-client-id').click();
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
  await page.goto(routes.base + routes.admin.openid, { waitUntil: 'networkidle' });
  await page
    .locator('div')
    .filter({
      hasText: new RegExp(`^${clientName}$`),
    })
    .click();
  await page.getByTestId('copy-client-id').click();
  const id = await getPageClipboard(page);
  await page.locator('.variant-copy').nth(1).click();
  const secret = await getPageClipboard(page);
  await context.close();
  return [id, secret];
};
