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
  await page.goto(routes.base + routes.admin.openid, { waitUntil: 'networkidle' });
  await page.getByTestId('add-openid-client').click();
  const modalElement = page.locator('#openid-client-modal');
  await modalElement.waitFor({ state: 'visible' });
  const modalForm = modalElement.locator('form');
  await modalForm.getByTestId('field-name').type(client.name);
  await modalForm.getByTestId('field-redirect_uri.0.url').type(client.redirectURL);
  for (const scope of client.scopes) {
    await modalForm.getByTestId(`field-scope-${scope}`).click();
  }
  const responsePromise = page.waitForResponse('**/oauth');
  await modalForm.locator('button[type="submit"]').click();
  const resp = await responsePromise;
  expect(resp.status()).toBe(201);
  await context.close();
};
