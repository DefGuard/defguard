import { Browser, expect } from 'playwright/test';

import { defaultUserAdmin } from '../../config';
import { waitForBase } from '../waitForBase';
import { waitForPromise } from '../waitForPromise';
import { loginBasic } from './login';

export const createGroup = async (browser: Browser, group_name: string) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.getByTestId('groups').click();
  await page.getByTestId('add-new-group').click();
  await page.getByTestId('field-name').fill(group_name);
  await page.getByTestId('next').click();
  await page.getByTestId('submit').click();
  await waitForPromise(1000);
  expect(page.locator(':text("' + group_name + '")')).toBeVisible();
};
