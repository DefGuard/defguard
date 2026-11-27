import { Browser, expect } from '@playwright/test';

import { routes } from '../../config';
import { User } from '../../types';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';

export const addApiToken = async (
  browser: Browser,
  user: Pick<User, 'username' | 'password'>,
  name: string,
): Promise<string> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, user);
  await page.goto(routes.base + routes.profile + user.username + routes.tab.api_tokens);
  await page.getByTestId('add-token').click();
  await page.getByTestId('field-name').fill(name);
  await page.getByTestId('submit').click();
  const apiToken = await page.getByTestId('copy-field').textContent();
  await page.getByTestId('close').click();
  await context.close();
  expect(apiToken).toBeTruthy();
  return apiToken as string;
};
