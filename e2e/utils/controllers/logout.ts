import { Page } from 'playwright';

export const logout = async (page: Page) => {
  await page.getByTestId('logout').click();
};
