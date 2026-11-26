import { Browser, expect } from '@playwright/test';

import { defaultUserAdmin, routes } from '../../config';
import { User } from '../../types';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';

type EnrollmentResponse = {
  user: User;
  token: string;
};

export const createUserEnrollment = async (
  browser: Browser,
  user: User,
  groups?: string[],
): Promise<EnrollmentResponse> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);

  await page.goto(routes.base + routes.identity.users);
  await page.getByTestId('add-user').click();
  await page.getByTestId('add-user-self-enrollment').click();
  const formElement = page.locator('[id="add-user-modal"]');
  await formElement.waitFor({ state: 'visible' });
  await formElement.getByTestId('field-username').fill(user.username);
  await formElement.getByTestId('field-first_name').fill(user.firstName);
  await formElement.getByTestId('field-last_name').fill(user.lastName);
  await formElement.getByTestId('field-email').fill(user.mail);
  await formElement.getByTestId('field-phone').fill(user.phone);
  await formElement.getByTestId('add-user-submit').click();
  const token = await formElement.getByTestId('activation-token-field').textContent();
  await formElement.locator('button[data-variant="primary"]').click();
  if (groups) {
    await page.goto(routes.base + routes.identity.users);
    const userRow = page.locator('.virtual-row').filter({ hasText: user.username });
    await userRow.locator('.icon-button').click();
    await page.getByTestId('edit-groups').click();
    for (const group of groups) {
      await page.locator('.item:has-text("' + group + '") .checkbox').click();
    }
    await page.locator('button:has-text("Submit")').click();

    for (const group of groups) {
      await expect(userRow).toContainText(group);
    }
  }
  if (!token) {
    throw new Error('No token');
  }

  await context.close();
  return { user, token };
};
