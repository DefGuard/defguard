import { Page } from 'playwright';

export const logout = async (page: Page) => {
  await page.getByTestId('avatar-icon').click();
  await page.getByTestId('logout').click();
  await page.waitForLoadState('load');
};
