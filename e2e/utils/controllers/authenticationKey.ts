import { Browser, expect } from '@playwright/test';

import { routes } from '../../config';
import { AuthenticationKeyType, User } from '../../types';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';

type AuthKeyForm = {
  name: string;
  key: string;
  keyType: AuthenticationKeyType;
};

export const addAuthKey = async (
  browser: Browser,
  user: Pick<User, 'username' | 'password'>,
  profileUsername: string,
  authKey: AuthKeyForm,
): Promise<void> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, user);
  await page.goto(
    routes.base + routes.profile + profileUsername + routes.tab.authentication_keys,
  );
  await page.getByRole('button', { name: 'Add new key', exact: true }).click();
  const modal = page.locator('#add-auth-key-modal');
  await modal.waitFor({ state: 'visible' });
  await page.locator('.box-track').click();
  const options = page.locator('.select-floating');
  await options.locator('.select-option').filter({ hasText: authKey.keyType }).click();
  if (await options.isVisible()) {
    await options.click();
  }
  await page.getByTestId('field-name').fill(authKey.name);
  await modal.locator('.field-box textarea').fill(authKey.key);
  const responsePromise = page.waitForResponse('**/auth_key');
  await page.getByTestId('add-key').click();
  expect((await responsePromise).status()).toBe(201);
  await modal.waitFor({ state: 'hidden' });
  await context.close();
};
