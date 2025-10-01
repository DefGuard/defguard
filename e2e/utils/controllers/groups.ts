import { Browser } from 'playwright';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';
import { defaultUserAdmin, routes } from '../../config';


export const createGroup = async (
  browser: Browser,
  is_admin: Boolean,
  group_name: string
): Promise<void> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.admin.groups);
  await page.getByTestId('add-group').click();
  await page.getByTestId('field-name').fill(group_name);
  await page.getByTestId('submit-group').click();
  await context.close();
};
