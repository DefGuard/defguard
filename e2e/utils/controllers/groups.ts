import { Browser, expect } from 'playwright/test';

import { defaultUserAdmin, routes } from '../../config';
import { waitForBase } from '../waitForBase';
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
  await expect(page.locator(':text("' + group_name + '")')).toBeVisible();
  await context.close();
};

export const addUserToGroups = async (
  browser: Browser,
  username: string,
  groups: string[],
) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.identity.users);
  const userRow = page.locator('.virtual-row').filter({ hasText: username });
  await userRow.locator('.icon-button').click();
  await page.getByTestId('edit-groups').click();
  for (const group of groups) {
    await page.locator('.item:has-text("' + group + '") .checkbox').click();
  }
  await page.locator('button:has-text("Submit")').click();
  for (const group of groups) {
    await expect(userRow).toContainText(group);
  }
  await context.close();
};
