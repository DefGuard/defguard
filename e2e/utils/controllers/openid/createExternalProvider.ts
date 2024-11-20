import { Browser } from 'playwright';

import { defaultUserAdmin, routes } from '../../../config';
import { OpenIdClient } from '../../../types';
import { waitForBase } from '../../waitForBase';
import { loginBasic } from '../login';

export const CreateExternalProvider = async (browser: Browser, client: OpenIdClient) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.admin.settings, { waitUntil: 'networkidle' });
  await page.getByRole('button', { name: 'OpenID' }).click();
  await page.locator('.content-frame').click();
  await page.getByRole('button', { name: 'Custom' }).click();
  await page.getByTestId('field-base_url').fill('http://localhost:8000/');
  await page.getByTestId('field-client_id').fill(client.clientID || '');
  await page.getByTestId('field-client_secret').fill(client.clientSecret || '');
  await page.getByTestId('field-display_name').fill(client.name);
  await page.getByRole('button', { name: 'Save changes' }).click();
};
