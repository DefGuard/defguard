import { Browser } from 'playwright';

import { defaultUserAdmin, routes } from '../../config';
import { User } from '../../types';
import { waitForBase } from '../waitForBase';
import { waitForPromise } from '../waitForPromise';
import { loginBasic } from './login';

// create user via default admin on separate context
export const createUser = async (
  browser: Browser,
  user: User,
  groups?: string[],
): Promise<void> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.admin.users);
  await page.getByTestId('add-user').click();
  const formElement = page.getByTestId('add-user-form');
  await formElement.waitFor({ state: 'visible' });
  await formElement.getByTestId('field-username').type(user.username);
  await formElement.getByTestId('field-password').type(user.password);
  await formElement.getByTestId('field-first_name').type(user.firstName);
  await formElement.getByTestId('field-last_name').type(user.lastName);
  await formElement.getByTestId('field-email').type(user.mail);
  await formElement.getByTestId('field-phone').type(user.phone);
  await formElement.locator('button[type="submit"]').click();
  await formElement.waitFor({ state: 'hidden', timeout: 2000 });
  if (groups) {
    groups = groups.map((g) => g.toLocaleLowerCase());
    await page.goto(routes.base + routes.admin.users + `/${user.username}`, {
      waitUntil: 'networkidle',
    });
    await page.getByTestId('edit-user').click();
    await page.waitForLoadState('networkidle');
    await waitForPromise(2000);
    await page.getByTestId('groups-select').locator('.select-container').click();
    await waitForPromise(2000);
    for (const group of groups) {
      await page
        .locator('.select-floating-ui')
        .locator('.options-container')
        .locator(`button >> span:has-text("${group}")`)
        .click();
    }
    await page.getByTestId('user-edit-save').click();
  }
  await context.close();
};
