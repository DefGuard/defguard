import { Browser, expect } from '@playwright/test';

import { defaultUserAdmin, routes } from '../../../config';
import { NetworkForm } from '../../../types';
import { apiDeleteAllNetworks } from '../../api/networks';
import { waitForBase } from '../../waitForBase';
import { loginBasic } from '../login';

export const createNetwork = async (browser: Browser, network: NetworkForm) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);

  // Delete any existing networks (e.g. the dummy one from the template) so the
  // wizard appears and devices are assigned to the test network.
  await apiDeleteAllNetworks();

  await page.goto(routes.base + routes.admin.wizard);
  await page.getByTestId('setup-network').click();
  const navNext = page.getByTestId('wizard-next');
  await page.getByTestId('setup-option-manual').click();
  await navNext.click();

  // fill form
  for (const key of Object.keys(network).filter((key) => key !== 'location_mfa_mode')) {
    const field = page.getByTestId(`field-${key}`);
    await field.clear();
    await field.type(network[key]);
  }
  // select location MFA mode
  if (network.location_mfa_mode) {
    const mfaModeSelect = page.locator('div.location-mfa-mode-select');
    let mode: number;
    switch (network.location_mfa_mode) {
      case 'none':
        mode = 0;
        break;
      case 'internal':
        mode = 1;
        break;
      case 'external':
        mode = 2;
        break;
      default:
        mode = 0;
        break;
    }
    const mfaMode = mfaModeSelect.locator('div.location-mfa-mode').nth(mode);
    await mfaMode.click();
  }

  const responseCreateNetworkPromise = page.waitForResponse('**/network');
  await navNext.click();
  const response = await responseCreateNetworkPromise;
  expect(response.status()).toBe(201);
  await context.close();
};
