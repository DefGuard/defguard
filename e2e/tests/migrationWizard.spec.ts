import { expect, test } from '@playwright/test';

import { runMigrationWizard } from '../utils/controllers/wizards/runMigrationWizard';
import { dockerRestart } from '../utils/docker';

test.describe('Migration Wizard', () => {
  test.beforeEach(() => {
    // Restore DB to the pre-wizard migration snapshot before each test.
    dockerRestart();
  });

  test('completes the happy path (no locations) and lands on vpn-overview', async ({
    page,
  }) => {
    await runMigrationWizard(page);
    await expect(page).toHaveURL(/\/vpn-overview/);
  });
});
