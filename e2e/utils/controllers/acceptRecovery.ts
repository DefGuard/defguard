import { Page } from 'playwright';

import { getPageClipboard } from '../getPageClipboard';

// accepts recovery and returns codes
export const acceptRecovery = async (page: Page): Promise<string[] | undefined> => {
  try {
    await page.getByTestId('copy-recovery-codes').waitFor({ state: 'visible' });
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
