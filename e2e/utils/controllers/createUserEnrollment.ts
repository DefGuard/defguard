import { BrowserContext } from 'playwright';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { User } from '../../types';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';
import { logout } from './logout';
import { getPageClipboard } from '../getPageClipboard';

export const createUserEnrollment = async (
  context: BrowserContext,
  username: string,
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
  await formElement.getByTestId('field-first_name').type(user.firstName);
  await formElement.getByTestId('field-last_name').type(user.lastName);
  await formElement.getByTestId('field-email').type(user.mail);
  await formElement.getByTestId('field-phone').type(user.phone);
  await formElement.getByTestId('field-enable_enrollment').click();
  await formElement.locator('button[type="submit"]').click();
  const modalElement = page.locator('#start-enrollment-modal');
  await modalElement.waitFor({ state: 'visible' });
  const modalForm = modalElement.locator('form');
  await modalForm.getByTestId('field-email').type('Test@test.pl');
  await modalForm.locator('.toggle-option').nth(1).click();
  await modalForm.locator('button[type="submit"]').click();
  // Copy to clipboard
  await modalElement
    .locator('.content')
    .locator('.expandable-card')
    .locator('.top')
    .locator('.actions')
    .getByTestId('copy-enrollment-token')
    .click();
  await modalElement.locator('.content').locator('.actions').getByTestId('button-close-enrollment').click();
  await modalElement.waitFor({ state: 'hidden' });
  const response = await getPageClipboard(page);
  await logout(page);
  return response;
};
