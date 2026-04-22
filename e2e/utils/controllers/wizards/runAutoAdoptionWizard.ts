import type { Page } from '@playwright/test';

import { defaultUserAdmin, testsConfig } from '../../../config';

// Run the full auto adoption wizard happy path.
// Assumes core is in auto-adoption mode (DEFGUARD_ADOPT_EDGE + DEFGUARD_ADOPT_GATEWAY
// are set) and the adoption API has already reported success for all components.
export const runAutoAdoptionWizard = async (page: Page) => {
  page.setDefaultTimeout(testsConfig.TEST_TIMEOUT * 1000);

  await page.goto(testsConfig.BASE_URL);

  await page
    .getByRole('button', { name: 'Start Defguard configuration' })
    .waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Start Defguard configuration' }).click();

  // Admin user step.
  await page.getByTestId('field-first_name').waitFor({ state: 'visible' });
  await page.getByTestId('field-first_name').fill(defaultUserAdmin.firstName);
  await page.getByTestId('field-last_name').fill(defaultUserAdmin.lastName);
  await page.getByTestId('field-username').fill(defaultUserAdmin.username);
  await page.getByTestId('field-email').fill(defaultUserAdmin.mail);
  await page.getByTestId('field-password').fill(defaultUserAdmin.password);
  const adminResp = page.waitForResponse(
    (r) => r.url().includes('/initial_setup/admin') && r.request().method() === 'POST',
  );
  await page.getByRole('button', { name: 'Continue' }).click();
  await adminResp;

  // Internal URL settings.
  await page.getByTestId('field-defguard_url').waitFor({ state: 'visible' });
  await page
    .getByTestId('field-defguard_url')
    .fill(testsConfig.CORE_BASE_URL.replace('/api/v1', ''));
  await page.getByRole('button', { name: 'Continue' }).click();

  // Internal URL SSL config
  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  // External URL settings.
  await page.getByTestId('field-public_proxy_url').waitFor({ state: 'visible' });
  await page.getByTestId('field-public_proxy_url').fill(testsConfig.ENROLLMENT_URL);
  await page.getByRole('button', { name: 'Continue' }).click();

  // External URL SSL config
  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  // VPN settings.
  await page.getByTestId('field-vpn_public_ip').waitFor({ state: 'visible' });
  await page.getByTestId('field-vpn_public_ip').fill('10.0.0.1');
  await page.getByTestId('field-vpn_wireguard_port').fill('51820');
  await page.getByTestId('field-vpn_gateway_address').fill('10.10.10.1/24');
  const vpnResp = page.waitForResponse(
    (r) =>
      r.url().includes('/initial_setup/auto_wizard/vpn_settings') &&
      r.request().method() === 'POST',
  );
  await page.getByRole('button', { name: 'Continue' }).click();
  await vpnResp;

  // MFA setup
  await page.getByText('Do not enforce MFA').waitFor({ state: 'visible' });
  const mfaResp = page.waitForResponse(
    (r) =>
      r.url().includes('/initial_setup/auto_wizard/mfa_settings') &&
      r.request().method() === 'POST',
  );
  await page.getByRole('button', { name: 'Continue' }).click();
  await mfaResp;

  // Summary
  await page
    .getByRole('button', { name: 'Go to Defguard' })
    .waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Go to Defguard' }).click();

  await page.waitForURL('**/vpn-overview', { timeout: testsConfig.TEST_TIMEOUT * 1000 });
};
