import { Page } from 'playwright';

export const logout = async (page: Page) => {
  await page.locator('#top-bar-profile').click();
  await page.getByTestId('logout').click();
  await page.waitForLoadState('load');
};
