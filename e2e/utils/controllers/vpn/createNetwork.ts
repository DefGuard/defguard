import { expect } from '@playwright/test';
import { BrowserContext } from 'playwright';

import { defaultUserAdmin, routes } from '../../../config';
import { NetworkForm } from '../../../types';
import { loginBasic } from '../login';

export const createNetwork = async (context: BrowserContext, network: NetworkForm) => {
  const page = await context.newPage();
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.admin.wizard);
  await page.getByTestId('setup-network').click();
  const navNext = page.getByTestId('wizard-next');
  await page.getByTestId('setup-option-manual').click();
  await navNext.click();
  for (const key of Object.keys(network)) {
    const field = page.getByTestId(`field-${key}`);
    await field.clear();
    await field.type(network[key]);
  }
  const responseCreateNetworkPromise = page.waitForResponse('**/network');
  await navNext.click();
  const response = await responseCreateNetworkPromise;
  expect(response.status()).toBe(201);
  await page.close();
};
