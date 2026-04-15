import { Page } from '@playwright/test';

import { routes } from '../config';
import { waitForPromise } from './waitForPromise';

// Retry navigation to the login page until it succeeds (e.g. after dockerRestart).
export const waitForBase = async (page: Page): Promise<void> => {
  let err = true;
  while (err) {
    try {
      await page.goto(routes.base + routes.auth.login, {
        waitUntil: 'load',
        timeout: 10000,
      });
      err = false;
    } catch {
      await waitForPromise(500);
    }
  }
};
