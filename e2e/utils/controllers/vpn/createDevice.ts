import { expect } from '@playwright/test';
import { BrowserContext } from 'playwright';

import { routes } from '../../../config';
import { DeviceForm, User } from '../../../types';
import { waitForRoute } from '../../waitForRoute';
import { loginBasic } from '../login';

export const createDevice = async (
  context: BrowserContext,
  user: User,
  device: DeviceForm
) => {
  const page = await context.newPage();
  await loginBasic(page, user);
  await page.goto(routes.base + routes.me);
  await page.getByTestId('add-device').click();
  await waitForRoute(page, routes.addDevice);
  // chose manual
  const choiceCard = page.locator('#setup-method-step');
  await choiceCard.waitFor({ state: 'visible' });
  await choiceCard.getByTestId('choice-manual').click();
  await page.getByTestId('next-step').click();
  const configStep = page.locator('#add-device-setup-step');
  await configStep.waitFor({ state: 'visible' });
  // fill form
  await configStep.getByTestId('field-name').clear();
  await configStep.getByTestId('field-name').type(device.name);
  if (device.pubKey && device.pubKey.length) {
    await page.locator('.toggle-option').nth(1).click();
    await configStep.getByTestId('field-publicKey').clear();
    await configStep.getByTestId('field-publicKey').type(device.pubKey);
  }
  // await response
  const responsePromise = page.waitForResponse('**/device/**');
  await page.getByTestId('next-step').click();
  const response = await responsePromise;
  expect(response.status()).toBe(201);
  await page.close();
};
