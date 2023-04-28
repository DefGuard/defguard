import { Page } from 'playwright';

import { routes } from '../../config';
import { User } from '../../types';

export const createUser = async (page: Page, user: User) => {
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
};
