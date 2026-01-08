import { Browser } from 'playwright';

import { defaultUserAdmin, routes } from '../../config';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';

export const createWebhook = async (
  browser: Browser,
  url: string,
  description: string,
  secret_token?: string,
): Promise<void> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.webhooks);
  await page.getByTestId('add-new-webhook').click();
  await page.getByTestId('field-url').fill(url);
  await page.getByTestId('field-description').fill(description);
  if (secret_token) {
    await page.getByTestId('field-token').fill(secret_token);
  } else {
    await page.getByTestId('field-token').fill('   ');
  }
  await page.getByTestId('field-on_user_created').click();
  await page.getByTestId('submit').click();

  await context.close();
};
