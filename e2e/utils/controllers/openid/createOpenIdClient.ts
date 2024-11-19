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
  const urls = client.redirectURL.length;
  for (let i = 0; i < urls; i++) {
    const isLast = i === urls - 1;
    await modalForm
      .getByTestId(`field-redirect_uri.${i}.url`)
      .fill(client.redirectURL[i]);
    if (!isLast) {
      await modalForm.locator('button:has-text("Add URL")').click();
    }
  }
  for (const scope of client.scopes) {
    await modalForm.getByTestId(`field-scope-${scope}`).click();
  }
  const responsePromise = page.waitForResponse('**/oauth');
  await modalForm.locator('button[type="submit"]').click();
  const resp = await responsePromise;
  expect(resp.status()).toBe(201);
  await context.close();
};
