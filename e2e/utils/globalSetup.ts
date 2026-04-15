import { chromium, request } from '@playwright/test';

import { defaultUserAdmin, testsConfig } from '../config';
import { dockerCheckContainers, dockerCreateSnapshot, dockerUp } from './docker';
import { loadEnv } from './loadEnv';
import { waitForPromise } from './waitForPromise';

const setLicense = async () => {
  const license = process.env.DEFGUARD_LICENSE_KEY;
  if (!license) return;

  const ctx = await request.newContext({ baseURL: testsConfig.BASE_URL });

  const authRes = await ctx.post('/api/v1/auth', {
    data: {
      username: defaultUserAdmin.username,
      password: defaultUserAdmin.password,
    },
  });
  if (!authRes.ok()) {
    await ctx.dispose();
    throw new Error(`Auth failed with status ${authRes.status()}`);
  }

  // defguard_session cookie is automatically stored in the context
  const patchRes = await ctx.patch('/api/v1/settings', {
    data: { license: license.trim() },
  });
  if (!patchRes.ok()) {
    await ctx.dispose();
    throw new Error(`Setting license failed with status ${patchRes.status()}`);
  }

  await ctx.dispose();
  console.log('License key set.');
};

const waitForCore = async () => {
  const { default: http } = await import('http');
  const coreUrl = new URL(
    testsConfig.CORE_BASE_URL.replace('/api/v1', '') + '/api/v1/health',
  );
  await new Promise<void>((resolve) => {
    const check = () => {
      const req = http.get(coreUrl.toString(), (res) => {
        if (res.statusCode && res.statusCode < 500) {
          resolve();
        } else {
          setTimeout(check, 2000);
        }
      });
      req.on('error', () => setTimeout(check, 2000));
      req.end();
    };
    check();
  });
};
const runWizard = async () => {
  const browser = await chromium.launch({ headless: !process.env.HEADED });
  const context = await browser.newContext();
  const page = await context.newPage();
  // Inherit the same timeout used by tests so wizard steps don't time out early on slow CI.
  page.setDefaultTimeout(testsConfig.TEST_TIMEOUT * 1000);

  // Navigate to base URL — app redirects to wizard if setup not done
  await page.goto(testsConfig.BASE_URL);

  // Step 1: Click "Configure Defguard"
  await page
    .getByRole('button', { name: 'Configure Defguard' })
    .waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Configure Defguard' }).click();

  // Step 2: Fill admin user form
  await page.getByTestId('field-first_name').waitFor({ state: 'visible' });
  await page.getByTestId('field-first_name').fill(defaultUserAdmin.firstName);
  await page.getByTestId('field-last_name').fill(defaultUserAdmin.lastName);
  await page.getByTestId('field-username').fill(defaultUserAdmin.username);
  await page.getByTestId('field-email').fill(defaultUserAdmin.mail);
  await page.getByTestId('field-password').fill(defaultUserAdmin.password);

  // Step 3: Continue to next step
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 4: Fill Defguard URL and proxy URL
  await page.getByTestId('field-defguard_url').waitFor({ state: 'visible' });
  await page
    .getByTestId('field-defguard_url')
    .fill(testsConfig.CORE_BASE_URL.replace('/api/v1', ''));
  await page.getByTestId('field-public_proxy_url').fill(testsConfig.ENROLLMENT_URL);

  // Continue to CA step
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 5: Click "Create a certificate authority..." option (recommended)
  await page.locator('.interactive-content').first().waitFor({ state: 'visible' });
  await page.locator('.interactive-content').first().click();

  // Fill CA fields
  await page.getByTestId('field-ca_common_name').waitFor({ state: 'visible' });
  await page.getByTestId('field-ca_common_name').fill('Defguard Test CA');
  await page.getByTestId('field-ca_email').fill('ca@defguard.test');

  // Continue
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 6: CA summary — Continue
  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 7: Confirm Edge deployment checkbox + Next
  await page.locator('.checkbox').waitFor({ state: 'visible' });
  await page.locator('.checkbox').click();
  await page.getByRole('button', { name: 'Next' }).click();

  // Step 8: Edge component — fill name and IP
  await page.getByTestId('field-common_name').waitFor({ state: 'visible' });
  await page.getByTestId('field-common_name').fill('edge-test');
  await page.getByTestId('field-ip_or_domain').fill('proxy');

  // Adopt Edge component
  await page.getByRole('button', { name: 'Adopt Edge component' }).click();

  // Step 9: Edge adoption — Continue
  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 10: "I'll do this later"
  await page
    .getByRole('button', { name: "I'll do this later" })
    .waitFor({ state: 'visible' });
  await page.getByRole('button', { name: "I'll do this later" }).click();

  await context.close();
  await browser.close();
};

export default async function globalSetup() {
  loadEnv();

  if (!dockerCheckContainers()) {
    dockerUp();
  }

  // Wait until core HTTP is ready before running the wizard.
  console.log('Waiting for Defguard Core to be ready...');
  await waitForCore();
  console.log('Core is ready. Running setup wizard...');

  await runWizard();

  await waitForPromise(3000);
  await setLicense();

  // Overwrite the snapshot with post-wizard state.
  dockerCreateSnapshot();
}
