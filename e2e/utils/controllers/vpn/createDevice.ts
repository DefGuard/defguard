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
  const modal = await page.locator('#add-user-device-modal');
  await modal.locator('.fold-button').click();
  await modal.getByTestId('client-manual').click();
  await modal.getByTestId('field-name').fill(device.name);
  if (device.pubKey && device.pubKey.length) {
    await modal.getByTestId('field-genChoice-manual').click();
    await modal.getByTestId('field-publicKey').fill(device.pubKey);
  }
  // const responsePromise = page.waitForResponse('**/device/**'); //TODO: broken for now.
  await modal.getByTestId('continue').click();
  // const response = await responsePromise;
  // expect(response.status()).toBe(201);
  await context.close();
};
