import { Page } from 'playwright';

import { routes } from '../../config';
import { getPageClipboard } from '../getPageClipboard';
import { waitForPromise } from '../waitForPromise';

// accepts recovery and returns codes
export const acceptRecovery = async (page: Page): Promise<string[] | undefined> => {
  try {
    await waitForPromise(2000);

    await page.getByTestId('copy-recovery-codes').click();
    const recoveryString = await getPageClipboard(page);
    const recovery = recoveryString.split('\n').filter((line) => line.trim());

    await page.getByTestId('confirm-code-save').click();
    await page.getByTestId('finish-recovery-codes').click();
    return recovery;
  } catch {
    return undefined;
  }
};
