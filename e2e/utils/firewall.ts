import { Browser } from 'playwright';

import { defaultUserAdmin, routes } from '../config';
import { Protocols } from '../types';
import { loginBasic } from './controllers/login';
import { waitForBase } from './waitForBase';

export const createAlias = async (
  browser: Browser,
  name: string,
  addresses?: string[],
  ports?: string[],
  protocols?: Protocols[],
): Promise<void> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.firewall.aliases);
  await page.getByTestId('add-alias').click();
  await page.waitForURL(routes.base + routes.firewall.addAlias + '**');
  await page.getByTestId('field-name').fill(name);

  if (addresses) {
    for (const address of addresses) {
      await page.getByTestId('field-addresses').fill(address);
      await page.getByTestId('field-addresses').press('Enter');
    }
  }

  if (ports) {
    await page.getByTestId('field-ports').fill(ports.join(','));
  }

  if (protocols) {
    for (const protocol of protocols) {
      await page.locator('.values-tack .item').filter({ hasText: protocol }).click();
    }
  }
  await page.locator('button[type="submit"]').click();
  await page.waitForURL(routes.base + routes.firewall.aliases + '**');
  await context.close();
};
