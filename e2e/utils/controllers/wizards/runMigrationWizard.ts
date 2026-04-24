import type { Page } from '@playwright/test';

import { defaultUserAdmin, testsConfig } from '../../../config';

// Run the migration wizard happy path (no locations to migrate).
// Assumes core is in migration mode (admin user exists, no wizard row in DB)
// and the user is not yet authenticated.
export const runMigrationWizard = async (page: Page) => {
  page.setDefaultTimeout(testsConfig.TEST_TIMEOUT * 1000);

  // The migration wizard route requires authentication.
  await page.goto(testsConfig.BASE_URL);
  await page.getByTestId('field-username').waitFor({ state: 'visible' });
  await page.getByTestId('field-username').fill(defaultUserAdmin.username);
  await page.getByTestId('field-password').fill(defaultUserAdmin.password);
  await page.getByTestId('sign-in').click();

  // Welcome screen
  await page
    .getByRole('button', { name: 'Start migration process' })
    .waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Start migration process' }).click();

  // General configuration
  await page.getByTestId('field-default_admin_group_name').waitFor({ state: 'visible' });
  const generalResp = page.waitForResponse(
    (r) => r.url().includes('/api/v1/settings') && r.request().method() === 'PATCH',
  );
  await page.getByRole('button', { name: 'Continue' }).click();
  await generalResp;

  // CA configuration.
  await page.getByTestId('field-ca_common_name').waitFor({ state: 'visible' });
  await page.getByTestId('field-ca_common_name').fill('Migration Test CA');
  await page.getByTestId('field-ca_email').fill('ca@migration.test');
  const caResp = page.waitForResponse(
    (r) => r.url().includes('/migration/ca') && r.request().method() === 'POST',
  );
  await page.getByRole('button', { name: 'Continue' }).click();
  await caResp;

  // CA summary
  await page
    .getByRole('button', { name: 'Download CA certificate' })
    .waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  // Edge deployment
  const edgeDeployCheckbox = page.getByRole('button', {
    name: 'Confirm that you have deployed Edge',
  });
  await edgeDeployCheckbox.waitFor({ state: 'visible' });
  await edgeDeployCheckbox.click();
  await page.getByRole('button', { name: 'Continue' }).click();

  // Edge component
  await page.getByTestId('field-common_name').waitFor({ state: 'visible' });
  await page.getByTestId('field-common_name').fill('edge-test');
  await page.getByTestId('field-ip_or_domain').fill('edge');
  await page.getByRole('button', { name: 'Adopt Edge component' }).click();

  // Edge adoption
  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.waitForFunction(
    () =>
      [...document.querySelectorAll<HTMLButtonElement>('button')].some(
        (b) => b.textContent?.trim() === 'Continue' && !b.disabled,
      ),
    { timeout: 120000 },
  );
  await page.getByRole('button', { name: 'Continue' }).click();

  // Internal URL settings
  await page.getByTestId('field-defguard_url').waitFor({ state: 'visible' });
  await page
    .getByTestId('field-defguard_url')
    .fill(testsConfig.CORE_BASE_URL.replace('/api/v1', ''));
  const internalUrlResp = page.waitForResponse(
    (r) =>
      r.url().includes('/migration/internal_url_settings') &&
      r.request().method() === 'POST',
  );
  await page.getByRole('button', { name: 'Continue' }).click();
  await internalUrlResp;

  // Internal URL SSL config
  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  // External URL settings
  await page.getByTestId('field-public_proxy_url').waitFor({ state: 'visible' });
  await page.getByTestId('field-public_proxy_url').fill(testsConfig.ENROLLMENT_URL);
  const externalUrlResp = page.waitForResponse(
    (r) =>
      r.url().includes('/migration/external_url_settings') &&
      r.request().method() === 'POST',
  );
  await page.getByRole('button', { name: 'Continue' }).click();
  await externalUrlResp;

  // External URL SSL config
  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  // Confirmation
  await page
    .getByRole('button', { name: 'Go to Defguard' })
    .waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Go to Defguard' }).click();

  await page.waitForURL('**/vpn-overview', { timeout: testsConfig.TEST_TIMEOUT * 1000 });
};
