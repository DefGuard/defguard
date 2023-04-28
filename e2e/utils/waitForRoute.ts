import { Page } from 'playwright';

export const waitForRoute = async (page: Page, route: string) => {
  await page.waitForURL('**' + route);
};
