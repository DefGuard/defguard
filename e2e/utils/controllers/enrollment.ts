import { expect, Page } from '@playwright/test';
import { BrowserContext } from 'playwright';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { User } from '../../types';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';
import { logout } from './logout';

export const password = 'TestEnrollment1234!!';

export const createUserEnrollment = async (
  context: BrowserContext,
  username: string
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
  await modalElement
    .locator('.content')
    .locator('.actions')
    .getByTestId('button-close-enrollment')
    .click();
  await modalElement.waitFor({ state: 'hidden' });
  await logout(page);
  return user;
};

export const setToken = async (token: string, page: Page) => {
  const formElement = page.getByTestId('enrollment-token-form');
  await formElement.getByTestId('field-token').type(token);
  await formElement.locator('button[type="submit"]').click();
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
  await formElement.getByTestId('field-password').type(password);
  await formElement.getByTestId('field-repeat').type(password);
};

export const createDevice = async (page: Page) => {
  const formElement = page.getByTestId('enrollment-device-form');
  await formElement.getByTestId('field-name').type('test');
  await formElement.locator('button[type="submit"]').click();
};
