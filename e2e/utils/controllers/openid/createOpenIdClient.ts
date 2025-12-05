import { expect } from '@playwright/test';
import { Browser } from 'playwright';

import { defaultUserAdmin, routes } from '../../../config';
import { OpenIdClient } from '../../../types';
import { waitForBase } from '../../waitForBase';
import { loginBasic } from '../login';

export const CreateOpenIdClient = async (browser: Browser, client: OpenIdClient) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.openid_apps, { waitUntil: 'networkidle' });
  await page.getByTestId('add-new-app').click();
  await page.getByTestId('field-name').fill(client.name);

  for (const idx in client.redirectURL) {
    page.getByTestId('field-redirect_uri[' + idx + ']').fill(client.redirectURL[idx]);
    if (Number(idx) + 1 < client.redirectURL.length) {
      page.getByTestId('add-url').click();
    }
  }

  for (const scope of client.scopes) {
    await page.getByTestId(`field-scope-${scope}`).click();
  }
  await page.getByTestId('save-settings').click();
  const responsePromise = page.waitForResponse('**/oauth');

  const resp = await responsePromise;
  expect(resp.status()).toBe(201);
  await context.close();
};
