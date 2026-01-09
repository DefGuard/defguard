import { Browser } from 'playwright';
import { expect } from 'playwright/test';

import { defaultUserAdmin, routes } from '../../config';
import { User } from '../../types';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';

export const enableUser = async (browser: Browser, user: User): Promise<void> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.identity.users);
  const userRow = page.locator('.virtual-row').filter({ hasText: user.username });
  await userRow.locator('.icon-button').click();
  await page.getByTestId('change-account-status').click();
  await expect(userRow).toContainText('Active');
  await context.close();
};

export const disableUser = async (browser: Browser, user: User): Promise<void> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.identity.users);
  const userRow = page.locator('.virtual-row').filter({ hasText: user.username });
  await userRow.locator('.icon-button').click();
  await page.getByTestId('change-account-status').click();
  await expect(userRow).toContainText('Disabled');
  await context.close();
};
