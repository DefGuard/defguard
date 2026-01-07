import { Page } from 'playwright';

export const getPageClipboard = async (page: Page) =>
  page.evaluate(() => {
    return navigator.clipboard.readText();
  });
