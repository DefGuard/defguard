import { chromium, request } from '@playwright/test';

import { defaultUserAdmin, testsConfig } from '../config';
import {
  dockerCheckContainers,
  dockerCheckTemplateExists,
  dockerCreateSnapshot,
  dockerUp,
} from './docker';
import { loadEnv } from './loadEnv';

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

export const waitForCore = async () => {
  const { default: http } = await import('http');
  const coreUrl = new URL(
    testsConfig.CORE_BASE_URL.replace('/api/v1', '') + '/api/v1/health',
  );
  await new Promise<void>((resolve) => {
    const check = () => {
      const req = http.get(coreUrl.toString(), (res) => {
        // Require exactly 200 — setup server may return other codes during transition.
        if (res.statusCode === 200) {
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

export const runWizard = async () => {
  const browser = await chromium.launch({ headless: !process.env.HEADED });
  const context = await browser.newContext();
  const page = await context.newPage();
  page.setDefaultTimeout(testsConfig.TEST_TIMEOUT * 1000);

  await page.goto(testsConfig.BASE_URL);

  await page
    .getByRole('button', { name: 'Configure Defguard' })
    .waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Configure Defguard' }).click();

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

  await page.getByTestId('field-default_admin_group_name').waitFor({ state: 'visible' });
  const generalConfigResp = page.waitForResponse(
    (r) =>
      r.url().includes('/initial_setup/general_config') &&
      r.request().method() === 'POST',
  );
  await page.getByRole('button', { name: 'Continue' }).click();
  await generalConfigResp;

  await page.getByTestId('field-ca_common_name').waitFor({ state: 'visible' });
  await page.getByTestId('field-ca_common_name').fill('Defguard Test CA');
  await page.getByTestId('field-ca_email').fill('ca@defguard.test');
  const caResp = page.waitForResponse(
    (r) => r.url().includes('/initial_setup/ca') && r.request().method() === 'POST',
  );
  await page.getByRole('button', { name: 'Continue' }).click();
  await caResp;

  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  await page.locator('.checkbox').waitFor({ state: 'visible' });
  await page.locator('.checkbox').click();
  await page.getByRole('button', { name: 'Continue' }).click();

  await page.getByTestId('field-common_name').waitFor({ state: 'visible' });
  await page.getByTestId('field-common_name').fill('edge-test');
  await page.getByTestId('field-ip_or_domain').fill('edge');

  // Adopt Edge component
  await page.getByRole('button', { name: 'Adopt Edge component' }).click();

  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  await page.getByTestId('field-defguard_url').waitFor({ state: 'visible' });
  await page
    .getByTestId('field-defguard_url')
    .fill(testsConfig.CORE_BASE_URL.replace('/api/v1', ''));
  await page.getByRole('button', { name: 'Continue' }).click();

  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  await page.getByTestId('field-public_proxy_url').waitFor({ state: 'visible' });
  await page.getByTestId('field-public_proxy_url').fill(testsConfig.ENROLLMENT_URL);
  await page.getByRole('button', { name: 'Continue' }).click();

  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  await page
    .getByRole('button', { name: "I'll do this later" })
    .waitFor({ state: 'visible' });
  await page.getByRole('button', { name: "I'll do this later" }).click();

  await page.waitForURL('**/vpn-overview', { timeout: testsConfig.TEST_TIMEOUT * 1000 });

  await context.close();
  await browser.close();
};

export default async function globalSetup() {
  loadEnv();

  if (!dockerCheckContainers()) {
    dockerUp();
  }

  if (dockerCheckTemplateExists()) {
    console.log('Snapshot already exists, skipping wizard.');
    return;
  }

  console.log('Waiting for Defguard Core to be ready...');
  await waitForCore();
  console.log('Core is ready. Running setup wizard...');

  await runWizard();

  console.log('Wizard complete. Waiting for main Core to be ready...');
  await waitForCore();
  console.log('Main Core is ready.');

  await setLicense();

  dockerCreateSnapshot();
}
