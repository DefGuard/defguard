import { BrowserContext } from 'playwright';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { User } from '../../types';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';
import { logout } from './logout';

export const createUser = async (
  context: BrowserContext,
  username: string,
  groups?: string[]
): Promise<User> => {
  const user: User = { ...testUserTemplate, username };
  const page = await context.newPage();
  await waitForBase(page);
  await page.goto(routes.base + routes.auth.login);
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
    await page.goto(routes.base + routes.admin.users + `/${user.username}`, {
      waitUntil: 'networkidle',
    });
    await page.getByTestId('edit-user').click();
    await page.waitForLoadState('networkidle');
    await page.getByTestId('groups-select').click();
    for (const group of groups) {
      await page
        .locator('.select-floating-ui')
        .getByRole('button', { name: group })
        .click();
    }
    await page.getByTestId('user-edit-save').click();
  }
  await logout(page);
  await page.close();
  return user;
};
