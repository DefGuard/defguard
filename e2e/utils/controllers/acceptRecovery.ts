import { Page } from 'playwright';

import { getPageClipboard } from '../getPageClipboard';

export const acceptRecovery = async (page: Page) => {
  const modalElement = page.locator('#view-recovery-codes');
  await modalElement.waitFor({ state: 'visible' });
  await page.getByTestId('copy-recovery').click();
  const codes = (await getPageClipboard(page)).split('\n');
  await page.getByTestId('accept-recovery').click();
  await modalElement.waitFor({ state: 'hidden' });
  return codes;
};
