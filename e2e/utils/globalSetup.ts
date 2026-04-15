import { chromium, request } from '@playwright/test';

import { defaultUserAdmin, testsConfig } from '../config';
import { dockerCheckContainers, dockerCreateSnapshot, dockerUp } from './docker';
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
        // Require exactly 200 — the setup server may return other 2xx/4xx
        // codes on this path during the setup-to-core transition.
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
const runWizard = async () => {
  const browser = await chromium.launch({ headless: !process.env.HEADED });
  const context = await browser.newContext();
  const page = await context.newPage();
  // Inherit the same timeout used by tests so wizard steps don't time out early on slow CI.
  page.setDefaultTimeout(testsConfig.TEST_TIMEOUT * 1000);

  // Navigate to base URL — app redirects to wizard if setup not done
  await page.goto(testsConfig.BASE_URL);

  // Step 1: Welcome — click "Configure Defguard"
  await page
    .getByRole('button', { name: 'Configure Defguard' })
    .waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Configure Defguard' }).click();

  // Step 2: Admin user form
  await page.getByTestId('field-first_name').waitFor({ state: 'visible' });
  await page.getByTestId('field-first_name').fill(defaultUserAdmin.firstName);
  await page.getByTestId('field-last_name').fill(defaultUserAdmin.lastName);
  await page.getByTestId('field-username').fill(defaultUserAdmin.username);
  await page.getByTestId('field-email').fill(defaultUserAdmin.mail);
  await page.getByTestId('field-password').fill(defaultUserAdmin.password);
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 3: General configuration — defaults are valid, continue
  await page.getByTestId('field-default_admin_group_name').waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 4: Certificate authority — form fields shown directly (no option selector)
  await page.getByTestId('field-ca_common_name').waitFor({ state: 'visible' });
  await page.getByTestId('field-ca_common_name').fill('Defguard Test CA');
  await page.getByTestId('field-ca_email').fill('ca@defguard.test');
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 5: CA summary — continue
  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 6: Edge deploy — check the confirmation checkbox to enable Continue
  await page.locator('.checkbox').waitFor({ state: 'visible' });
  await page.locator('.checkbox').click();
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 7: Edge component — fill name and IP/domain
  await page.getByTestId('field-common_name').waitFor({ state: 'visible' });
  await page.getByTestId('field-common_name').fill('edge-test');
  await page.getByTestId('field-ip_or_domain').fill('proxy');
  await page.getByRole('button', { name: 'Adopt Edge component' }).click();

  // Step 8: Edge adoption — wait for adoption to complete, then continue
  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 9: Internal URL settings — fill Defguard core URL, leave SSL as "none"
  await page.getByTestId('field-defguard_url').waitFor({ state: 'visible' });
  await page
    .getByTestId('field-defguard_url')
    .fill(testsConfig.CORE_BASE_URL.replace('/api/v1', ''));
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 10: Internal URL SSL config result — continue
  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 11: External URL settings — fill proxy/enrollment URL, leave SSL as "none"
  await page.getByTestId('field-public_proxy_url').waitFor({ state: 'visible' });
  await page.getByTestId('field-public_proxy_url').fill(testsConfig.ENROLLMENT_URL);
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 12: External URL SSL config result — continue
  await page.getByRole('button', { name: 'Continue' }).waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Continue' }).click();

  // Step 13: Confirmation — skip creating a location for now
  await page
    .getByRole('button', { name: "I'll do this later" })
    .waitFor({ state: 'visible' });
  await page.getByRole('button', { name: "I'll do this later" }).click();

  // Wait for navigation to /vpn-overview — this confirms that finishSetup()
  // completed on the backend (setup server has received its shutdown signal).
  await page.waitForURL('**/vpn-overview', { timeout: testsConfig.TEST_TIMEOUT * 1000 });

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

  // After the wizard finishes, the setup server shuts down and the main core
  // server takes over. Wait for the main core to be ready before proceeding.
  console.log('Wizard complete. Waiting for main Core to be ready...');
  await waitForCore();
  console.log('Main Core is ready.');

  await setLicense();

  // Overwrite the snapshot with post-wizard state.
  dockerCreateSnapshot();
}
