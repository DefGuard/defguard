import { Browser } from '@playwright/test';

import { defaultUserAdmin, routes } from '../../../config';
import { NetworkForm } from '../../../types';
import { waitForBase } from '../../waitForBase';
import { loginBasic } from '../login';

export const createRegularLocation = async (browser: Browser, network: NetworkForm) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.locations);
  await page.getByTestId('add-location').click();
  await page.getByTestId('add-regular-location').click();
  await page
    .locator('button[data-variant="primary"]')
    .filter({ hasText: 'Create new location' })
    .click();

  await page.getByTestId('field-name').fill(network.name);
  await page.getByTestId('field-endpoint').fill(network.endpoint);
  await page.getByTestId('field-port').fill(network.port);
  await page.getByTestId('continue').click();

  await page.getByTestId('field-address').fill(network.address);

  if (network.allowed_ips) {
    let addresses = '';
    for (const ip of network.allowed_ips) {
      addresses += ip + ',';
    }
    addresses = addresses.slice(0, -1);
    await page.getByTestId('field-allowed_ips').fill(addresses);
    await page.getByTestId('continue').click();
  }

  await page.getByTestId('continue').click();

  if (network.mfa_enabled) {
    await page.getByTestId('toggle-mfa').click();
  }
  await page.getByTestId('finish').click();

  await page.getByTestId('acl-continue').click();
  await page.getByTestId('posture-continue').click();
  await page.getByTestId('create-location').click();
  await page.locator('.icon-button .icon[data-kind="close"]').click();

  await context.close();
};

export const createServiceLocation = async (browser: Browser, network: NetworkForm) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.locations);
  await page.getByTestId('add-location').click();
  await page.getByTestId('add-service-location').click();
  await page
    .locator('button[data-variant="primary"]')
    .filter({ hasText: 'Create new location' })
    .click();

  await page.getByTestId('field-name').fill(network.name);
  await page.getByTestId('field-endpoint').fill(network.endpoint);
  await page.getByTestId('field-port').fill(network.port);

  await page.getByTestId('continue').click();

  await page.getByTestId('field-address').fill(network.address);

  if (network.allowed_ips) {
    let addresses = '';
    for (const ip of network.allowed_ips) {
      addresses += ip + ',';
    }
    addresses = addresses.slice(0, -1);
    await page.getByTestId('field-allowed_ips').fill(addresses);
    await page.getByTestId('continue').click();
  }
  await page.getByTestId('continue').click();
  await page.getByTestId('continue').click();
  await page.getByTestId('acl-continue').click();
  await page.getByTestId('posture-continue').click();
  await page.getByTestId('create-location').click();
  await page.locator('.icon-button .icon[data-kind="close"]').click();

  await context.close();
};
