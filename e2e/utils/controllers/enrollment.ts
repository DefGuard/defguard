import { Browser, expect, Page } from '@playwright/test';

import { defaultUserAdmin, routes } from '../../config';
import { User } from '../../types';
import { waitForBase } from '../waitForBase';
import { waitForPromise } from '../waitForPromise';
import { loginBasic } from './login';
import { logout } from './logout';

type EnrollmentResponse = {
  user: User;
  token: string;
};

export const createUserEnrollment = async (
  browser: Browser,
  user: User,
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
  await formElement.getByTestId('field-username').fill(user.username);
  await formElement.getByTestId('field-first_name').fill(user.firstName);
  await formElement.getByTestId('field-last_name').fill(user.lastName);
  await formElement.getByTestId('field-email').fill(user.mail);
  await formElement.getByTestId('field-phone').fill(user.phone);
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
  const tokenDiv = tokenStep.locator('.copy-field.spacer').nth(1); // field with token
  const tokenP = tokenDiv.locator('p.display-element');
  const token = await tokenP.textContent();
  if (typeof token !== 'string') {
    throw Error('Enrollment token not found');
  }
  expect(token.length).toBeGreaterThan(0);
  // close modal
  await modalElement.locator('.controls button.cancel').click();
  await modalElement.waitFor({ state: 'hidden' });
  // logout
  await logout(page);
  await context.close();
  return { user, token };
};

const startEnrollment = async (page: Page) => {
  const enrollmentLink = page.getByTestId('start-enrollment');
  await enrollmentLink.click();
  await page.waitForURL('**/enrollment-start', {
    waitUntil: 'networkidle',
  });
};

const startPasswordReset = async (page: Page) => {
  const passwordLink = page.getByTestId('start-password-reset');
  await passwordLink.click();
  await page.waitForURL('**/password', {
    waitUntil: 'networkidle',
  });
};

const fillPasswordResetStartForm = async (email: string, page: Page) => {
  const emailField = page.getByTestId('field-email');
  await emailField.waitFor({
    state: 'visible',
  });
  await emailField.pressSequentially(email);
};

const fillPasswordResetForm = async (
  {
    password,
    repeat,
  }: {
    password: string;
    repeat: string;
  },
  page: Page,
) => {
  const passwordElement = page.getByTestId('field-password');
  const repeatElement = page.getByTestId('field-repeat');
  await passwordElement.waitFor({
    state: 'visible',
  });
  await passwordElement.pressSequentially(password);
  await repeatElement.pressSequentially(repeat);
};

const confirmClientDownloadModal = async (page: Page) => {
  await page.getByTestId('modal-confirm-download-submit').click();
};

const navNext = async (page: Page) => {
  await page.getByTestId('page-nav-next').click();
};

const navBack = async (page: Page) => {
  await page.getByTestId('page-nav-back').click();
};

const fillTokenForm = async (token: string, page: Page) => {
  const tokenField = page.getByTestId('field-token');
  await tokenField.waitFor({
    state: 'visible',
  });
  await tokenField.pressSequentially(token);
};

const enrollmentController = {
  startEnrollment,
  startPasswordReset,
  fillPasswordResetForm,
  fillPasswordResetStartForm,
  fillTokenForm,
  confirmClientDownloadModal,
  navBack,
  navNext,
};

export default enrollmentController;
