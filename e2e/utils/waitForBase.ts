import { Page } from '@playwright/test';

import { routes } from '../config';
import { waitForPromise } from './waitForPromise';

// Sometimes test cant react front at the beginning of the test, this is a workaround
export const waitForBase = async (page: Page): Promise<void> => {
  let err = true;
  while (err) {
    try {
      await page.goto(routes.base + routes.auth.login, {
        waitUntil: 'networkidle',
      });
      err = false;
    } catch {
      await waitForPromise(500);
    }
  }
};
