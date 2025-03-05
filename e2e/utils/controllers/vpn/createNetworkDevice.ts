import { Browser, expect, Locator, Page } from '@playwright/test';

import { routes } from '../../../config';
import { EditNetworkDeviceForm, NetworkDeviceForm, User } from '../../../types';
import { waitForRoute } from '../../waitForRoute';
import { loginBasic } from '../login';

export const getDeviceRow = async ({
  page,
  deviceName,
}: {
  page: Page;
  deviceName: string;
}) => {
  const deviceList = await page.locator('#devices-page-devices-list').first();
  const deviceRows = await deviceList.locator('.device-row').all();
  const row = deviceRows.find(async (val) => {
    if ((await val.innerText()) === deviceName) {
      return true;
    } else {
      return false;
    }
  });
  expect(row).toBeDefined();
  return row as Locator;
};

export const doAction = async ({
  page,
  deviceRow,
  action,
}: {
  page: Page;
  deviceRow: Locator;
  action: string;
}) => {
  await deviceRow.locator('.edit-button').click();
  const editMenu = await page.locator('.edit-button-floating-ui').first();
  await editMenu.getByRole('button', { name: action }).click();
};

export const createNetworkDevice = async (
  browser: Browser,
  user: User,
  device: NetworkDeviceForm,
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

export const startNetworkDeviceEnrollment = async (
  browser: Browser,
  user: User,
  device: NetworkDeviceForm,
) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await loginBasic(page, user);
  await page.goto(routes.base + routes.admin.devices);
  await page.getByRole('button', { name: 'Add new' }).click();
  const configCard = page.locator('#add-standalone-device-modal');
  await configCard.getByRole('button', { name: 'Next' }).click();
  const deviceNameInput = await configCard.getByTestId('field-name');
  await deviceNameInput.fill(device.name);
  if (device.description && device.description.length > 0) {
    const deviceDescriptionInput = await page.getByTestId('field-description');
    await deviceDescriptionInput.fill(device.description);
  }
  const responsePromise = page.waitForResponse('**/device/network');
  await page.getByRole('button', { name: 'Add Device' }).click();
  const response = await responsePromise;
  expect(response.status()).toBe(200);
  const tokenCommand = await page.locator('.expanded-content').innerText();
  await context.close();
  return tokenCommand;
};

export const editNetworkDevice = async (
  browser: Browser,
  user: User,
  currentDeviceName: string,
  device: EditNetworkDeviceForm,
) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await loginBasic(page, user);
  await page.goto(routes.base + routes.admin.devices);
  await waitForRoute(page, routes.admin.devices);
  const deviceRow = await getDeviceRow({ page, deviceName: currentDeviceName });
  await doAction({ page, deviceRow, action: 'Edit' });
  const configCard = page.locator('#edit-standalone-device-modal');
  if (device.name) {
    const input = configCard.getByTestId('field-name');
    await input.fill(device.name);
  }
  if (device.ip) {
    const input = configCard.getByTestId('field-modifiableippart');
    await input.fill(device.ip);
  }
  if (device.description) {
    const input = configCard.getByTestId('field-description');
    await input.fill(device.description);
  }
  const responsePromise = page.waitForResponse('**/device/network');
  await configCard.locator('button').filter({ hasText: 'Submit' }).click();
  const response = await responsePromise;
  expect(response.status()).toBe(200);
  await context.close();
};
