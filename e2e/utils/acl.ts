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
  const modal = await page.locator('.card');
  await modal.getByTestId('field-name').fill(name);

  if (addresses) {
    await modal.getByTestId('field-addresses').fill(addresses.join(','));
  }

  if (ports) {
    await modal.getByTestId('field-ports').fill(ports.join(','));
  }

  if (protocols) {
    for (const protocol of protocols) {
      await modal.locator('.values-tack .item').filter({ hasText: protocol }).click();
    }
  }
  await modal.locator('button[data-variant="primary"]').click();
  await context.close();
};

// export const createRule = async (
//   browser: Browser,
//   name: string,
//   addresses?: string[],
//   ports?: string[],
//   protocols?: Protocols[],
// ): Promise<void> => {
//   const context = await browser.newContext();
//   const page = await context.newPage();
//   await waitForBase(page);
//   //TODO
//   await context.close();
// };
