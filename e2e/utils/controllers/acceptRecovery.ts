import { Page } from 'playwright';

import { routes } from '../../config';
import { getPageClipboard } from '../getPageClipboard';

export const acceptRecovery = async (page: Page): Promise<string[]> => {
  const modalElement = page.locator('#view-recovery-codes');
  await modalElement.waitFor({ state: 'visible' });
  await page.getByTestId('copy-recovery').click();
  const codes = (await getPageClipboard(page)).split('\n');
  await page.getByTestId('accept-recovery').click();
  await modalElement.waitFor({ state: 'hidden' });
  await page.waitForURL(routes.base + routes.auth.login);
  return codes;
};
