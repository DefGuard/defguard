import { Browser } from 'playwright';

import { defaultUserAdmin, routes } from '../../config';
import { User } from '../../types';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';

export const enableUser = async (browser: Browser, user: User): Promise<void> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + '/admin/users/' + user.username);
  await page.getByTestId('edit-user').click();
  await page.getByTestId('status-select').locator('.select-container').click();
  await page.locator('.select-option:has-text("Active")').click();
  await page.getByTestId('user-edit-save').click();
  await context.close();
};

export const disableUser = async (browser: Browser, user: User): Promise<void> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + '/admin/users/' + user.username);
  await page.getByTestId('edit-user').click({force: true});
  await page.getByTestId('status-select').locator('.select-container').click();
  await page.locator('.select-option:has-text("Disabled")').click();
  await page.getByTestId('user-edit-save').click();
  await context.close();
};
