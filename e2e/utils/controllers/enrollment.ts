import { Browser, expect, Page } from '@playwright/test';

import { defaultUserAdmin, routes } from '../../config';
import { User } from '../../types';
import { getPageClipboard } from '../getPageClipboard';
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
  user: User
): Promise<EnrollmentResponse> => {
  const context = await browser.newContext();
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
  waitForPromise(2000);
  const modalElement = page.locator('#add-user-modal');
  const enrollmentForm = modalElement.getByTestId('start-enrollment-form');
  await enrollmentForm.locator('.toggle-option').nth(1).click();
  await enrollmentForm.locator('button[type="submit"]').click();
  waitForPromise(2000);
  // Copy to clipboard
  const tokenStep = modalElement.locator('#enrollment-token-step');
  await tokenStep.getByTestId('copy-enrollment-token').click();
  const token = await getPageClipboard(page);
  expect(token.length).toBeGreaterThan(0);
  // close modal
  await modalElement.locator('.controls button.cancel').click();
  await modalElement.waitFor({ state: 'hidden' });
  // logout
  await logout(page);
  await context.close();
  return { user, token };
};

export const selectEnrollment = async (page: Page) => {
  const selectButton = page.getByTestId('select-enrollment');
  selectButton.click();
};

export const setToken = async (token: string, page: Page) => {
  const formElement = page.getByTestId('enrollment-token-form');
  await formElement.getByTestId('field-token').type(token);
  await page.getByTestId('enrollment-token-submit-button').click();
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
