import { expect, test } from '@playwright/test';

import { runAutoAdoptionWizard } from '../utils/controllers/wizards/runAutoAdoptionWizard';
import { dockerRestartAutoAdoption } from '../utils/docker';

test.describe('Auto Adoption Wizard', () => {
  test.beforeEach(() => {
    // Restore DB to the pre-wizard snapshot before each test.
    dockerRestartAutoAdoption();
  });

  test('completes the happy path and lands on vpn-overview', async ({ page }) => {
    await runAutoAdoptionWizard(page);
    await expect(page).toHaveURL(/\/vpn-overview/);
  });
});
