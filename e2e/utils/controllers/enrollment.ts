import { Browser, expect, Page } from '@playwright/test';

import { defaultUserAdmin, routes, testsConfig } from '../../config';
import { User } from '../../types';
import { waitForBase } from '../waitForBase';
import { waitForPromise } from '../waitForPromise';
import { loginBasic } from './login';
import { logout } from './logout';

export const password = 'TestEnrollment1234!!';

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
  let token = await formElement.getByTestId('activation-token-field').textContent();
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

  return { user, token };
};

export const selectEnrollment = async (page: Page) => {
  const selectButton = page.getByTestId('select-enrollment');
  selectButton.click();
};

export const setToken = async (token: string, page: Page) => {
  await page.getByTestId('start-enrollment').click();
  await page.getByTestId('field-token').fill(token);
  await page.getByTestId('page-nav-next').click();
};

export const validateData = async (user: User, page: Page) => {
  const formElement = page
    .locator('#enrollment-data-verification-card')
    .getByTestId('enrollment-data-verification')
    .locator('.row');
  const firstName = await formElement.locator('p').nth(0).textContent();
  const lastName = await formElement.locator('p').nth(1).textContent();
  const mail = await formElement.locator('p').nth(2).textContent();
  const phone = await formElement.getByTestId('field-phone').inputValue();
  expect(firstName).toBe(user.firstName);
  expect(lastName).toBe(user.lastName);
  expect(mail).toBe(user.mail);
  expect(phone).toBe(user.phone);
};

export const setPassword = async (page: Page) => {
  const formElement = page.getByTestId('enrollment-password-form');
  await formElement.getByTestId('field-password').fill(password);
  await formElement.getByTestId('field-repeat').fill(password);
};

export const createDevice = async (page: Page) => {
  const formElement = page.getByTestId('enrollment-device-form');
  await formElement.getByTestId('field-name').fill('test');
  await formElement.locator('button[type="submit"]').click();
};
