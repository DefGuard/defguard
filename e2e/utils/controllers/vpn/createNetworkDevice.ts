import { Browser, expect } from '@playwright/test';

import { routes } from '../../../config';
import { NetworkDeviceForm, User } from '../../../types';
import { loginBasic } from '../login';

export const createNetworkDevice = async (
  browser: Browser,
  user: User,
  device: NetworkDeviceForm
) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await loginBasic(page, user);
  await page.goto(routes.base + routes.admin.devices);
  await page.getByRole('button', { name: 'Add new' }).click();
  const configCard = page.locator('#add-standalone-device-modal');
  await configCard.getByRole('button', { name: 'Select' }).click();
  await configCard.getByRole('button', { name: 'Next' }).click();
  const deviceNameInput = await configCard.getByTestId('field-name');
  await deviceNameInput.fill(device.name);
  if (device.description && device.description.length > 0) {
    const deviceDescriptionInput = await page.getByTestId('field-description');
    await deviceDescriptionInput.fill(device.description);
  }
  if (device.pubKey && device.pubKey.length) {
    await configCard.locator('.toggle-option').nth(1).click();
    const devicePublicKeyInput = await configCard.getByTestId('field-wireguard_pubkey');
    await devicePublicKeyInput.fill(device.pubKey);
  }
  const responsePromise = page.waitForResponse('**/device/network');
  await page.getByRole('button', { name: 'Add Device' }).click();
  const response = await responsePromise;
  expect(response.status()).toBe(201);
  await context.close();
};
