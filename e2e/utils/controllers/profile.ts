import { Page } from 'playwright';

import { routes } from '../../config';

export const changePassword = async (page: Page, currentPassword: string) => {
  await page.goto(routes.base + routes.me);
  await page.getByTestId('edit-user').click();
  await page.getByTestId('button-change-password').click();
  const formElement = page.getByTestId('change-self-password-form');
  await formElement.waitFor({ state: 'visible' });
  await formElement.getByTestId('field-old_password').type(currentPassword);
  const newPassword = 'Test1234#$%';
  await formElement.getByTestId('field-new_password').type(newPassword);
  await formElement.getByTestId('field-repeat').type(newPassword);
  await formElement.locator('button[type="submit"]').click();
  await formElement.waitFor({ state: 'hidden', timeout: 2000 });
  return newPassword;
};
