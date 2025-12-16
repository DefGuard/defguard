import { Browser, expect } from '@playwright/test';

import { routes } from '../../../config';
import { DeviceForm, User } from '../../../types';
import { loginBasic } from '../login';

export const createDevice = async (browser: Browser, user: User, device: DeviceForm) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await loginBasic(page, user);
  await page.goto(routes.base + routes.profile + user.username + routes.tab.devices);
  await page.getByTestId('add-device').click();
  await page.getByTestId('show-advanced-options').click();
  await page.getByTestId('client-manual').click();
  await page.getByTestId('field-name').fill(device.name);
  if (device.pubKey && device.pubKey.length) {
    await page.getByTestId('field-genChoice-manual').click();
    await page.getByTestId('field-publicKey').fill(device.pubKey);
  }
  // const responsePromise = page.waitForResponse('**/device/**'); //TODO: broken for now.
  await page.getByTestId('continue').click();
  // const response = await responsePromise;
  // expect(response.status()).toBe(201);
  await context.close();
};
