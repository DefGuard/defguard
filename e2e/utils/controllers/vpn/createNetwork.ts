import { Browser, expect } from '@playwright/test';

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

  await page.getByTestId('field-name').fill(network.name);
  await page.getByTestId('field-address').fill(network.endpoint);
  await page.getByTestId('field-port').fill(network.port);
  await page.getByTestId('continue').click();

  await page.getByTestId('field-endpoint').fill(network.address);

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

  if (network.location_mfa_mode) {
    switch (network.location_mfa_mode) {
      case 'internal':
        await page.getByTestId('enforce-internal-mfa').click();
        break;
      case 'external':
        await page.getByTestId('enforce-external-mfa').click();
        break;
      default:
        await page.getByTestId('do-not-enforce-mfa').click();
        break;
    }
  }
  await page.getByTestId('finish').click();

  await page.getByTestId('acl-continue').click();
  await page.getByTestId('create-location').click();

  await page.waitForURL('**/locations');

  await expect(page.url()).toBe(routes.base + routes.locations);
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

  await page.getByTestId('field-name').fill(network.name);
  await page.getByTestId('field-address').fill(network.endpoint);
  await page.getByTestId('field-port').fill(network.port);

  await page.getByTestId('continue').click();

  await page.getByTestId('field-endpoint').fill(network.address);

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
  await page.getByTestId('create-location').click();

  await page.waitForURL('**/locations');
  await expect(page.url()).toBe(routes.base + routes.locations);
  await context.close();
};
