import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { OpenIdClient } from '../types';
import { loginBasic } from '../utils/controllers/login';
import { CreateOpenIdClient } from '../utils/controllers/openid/createOpenIdClient';
import { getPageClipboard } from '../utils/getPageClipboard';
import { waitForPromise } from '../utils/waitForPromise';
import { waitForRoute } from '../utils/waitForRoute';

// FIXME containerize test client so tests can run without external testing client

test('Authorize openId client.', async ({ page }) => {
  const client: OpenIdClient = {
    name: 'test 01',
    redirectURL: 'https://oidcdebugger.com/debug',
    scopes: ['openid'],
  };
  await loginBasic(page, defaultUserAdmin);
  await waitForRoute(page, routes.admin.wizard);
  await CreateOpenIdClient(page, client);
  await page.getByTestId('edit-openid-client-1').click();
  await page.getByTestId('copy-openid-client-id').click();
  const clientId = await getPageClipboard(page);
  await page.goto('https://oidcdebugger.com/');
  await page.locator('#authorizeUri').type(routes.base + routes.authorize);
  await page.locator('#clientId').type(clientId);
  await page.locator('.debug__form-submit').click();
  await waitForRoute(page, routes.consent);
  await page.getByTestId('openid-allow').click();
  await waitForRoute(page, 'https://oidcdebugger.com/');
  await waitForPromise(2000);
  const headerMessage = await page
    .locator('.debug__callback-header')
    .locator('h1')
    .textContent();
  expect(headerMessage?.replace(' ', '')).toBe('Success!');
});
