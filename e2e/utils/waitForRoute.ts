import { Page } from 'playwright';

import { waitForPromise } from './waitForPromise';

export const waitForRoute = async (page: Page, route: string) => {
  let match = false;
  while (!match) {
    match = page.url().includes(route);
    await waitForPromise(50);
  }
};
