import { Browser, expect, Locator, Page } from '@playwright/test';

import { routes } from '../../../config';
import { EditNetworkDeviceForm, NetworkDeviceForm, User } from '../../../types';
import { waitForRoute } from '../../waitForRoute';
import { loginBasic } from '../login';
import { waitForPromise } from '../../waitForPromise';
import { getPageClipboard } from '../../getPageClipboard';

export const getDeviceRow = async ({
  page,
  deviceName,
}: {
  page: Page;
  deviceName: string;
}) => {
  const deviceList = page.locator('#devices-page-devices-list').first();
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
  const editMenu = page.locator('.edit-button-floating-ui').first();
  await editMenu.getByRole('button', { name: action }).click();
};

export const createNetworkCLIDevice = async (
  browser: Browser,
  user: User,
  device: NetworkDeviceForm,
) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await loginBasic(page, user);
  await page.goto(routes.base + routes.network_devices, {
    waitUntil: 'networkidle',
  });
  await page.getByTestId('add-device').click();
  await page.getByTestId('defguard-cli').click();
  await page.getByTestId('field-name').fill(device.name);
  if (device.description) {
    await page.getByTestId('field-description').fill(device.description);
  }
  await page.getByTestId('submit').click();
  await page.getByTestId('finish').click();
  await waitForPromise(1000);
  const deviceRow = page.locator('.virtual-row').filter({ hasText: device.name });
  await expect(deviceRow).toContainText('Awaiting Setup');
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
  await page.goto(routes.base + routes.network_devices);
  await page.getByTestId('add-device').click();
  await page.getByTestId('wireguard-client').click();

  await page.getByTestId('field-name').fill(device.name);
  if (device.description) {
    await page.getByTestId('field-description').fill(device.description);
  }
  if (device.pubKey) {
    await page.getByTestId('field-generateKeys-false').click();
    await page.getByTestId('field-wireguard_pubkey').fill(device.pubKey);
  }
  await page.getByTestId('submit').click();
  await waitForPromise(2000);
  await page.getByTestId('copy-config').click();

  await page.getByTestId('finish').click();

  const deviceRow = page.locator('.virtual-row').filter({ hasText: device.name });
  await expect(deviceRow).toContainText('Ready');

  const tokenCommand = await getPageClipboard(page);
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
