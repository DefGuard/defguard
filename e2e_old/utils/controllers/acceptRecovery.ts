import { Page } from 'playwright';

import { routes } from '../../config';
import { getPageClipboard } from '../getPageClipboard';

// accepts recovery and returns codes
export const acceptRecovery = async (page: Page): Promise<string[] | undefined> => {
  try {
    // if modal won't show up it means another method was already active and new codes won't show up
    const modalElement = page.locator('#view-recovery-codes');
    await modalElement.waitFor({ state: 'visible', timeout: 2000 });
    await page.getByTestId('copy-recovery').click();
    const codes = (await getPageClipboard(page)).split('\n');
    await page.getByTestId('accept-recovery').click();
    await modalElement.waitFor({ state: 'hidden' });
    await page.waitForURL(routes.base + routes.auth.login);
    return codes;
  } catch {
    return undefined;
  }
};
