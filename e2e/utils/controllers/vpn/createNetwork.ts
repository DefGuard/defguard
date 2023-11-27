import { Browser, expect } from '@playwright/test';

import { defaultUserAdmin, routes } from '../../../config';
import { NetworkForm } from '../../../types';
import { waitForBase } from '../../waitForBase';
import { loginBasic } from '../login';

export const createNetwork = async (browser: Browser, network: NetworkForm) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
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
  await context.close();
};
