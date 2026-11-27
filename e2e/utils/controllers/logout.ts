import { Page } from 'playwright';

export const logout = async (page: Page) => {
  await page.locator('#top-bar-profile').click();
  const responsePromise = page.waitForResponse(
    (resp) => resp.url().endsWith('/auth/logout') && resp.request().method() === 'POST',
  );
  await page.getByTestId('logout').click();
  await responsePromise;
  await page.waitForLoadState('load');
};
