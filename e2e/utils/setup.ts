import { chromium } from '@playwright/test';
import { execSync } from 'child_process';

import { defaultUserAdmin, testsConfig } from '../config';
import { loginBasic } from './controllers/login';
import { dockerCompose, dockerCreateTemplate, dockerUp, resetToFreshDb } from './docker';

// True only if the template exists AND has the marker network created by
// completeWizard. A template left over from an older harness (wizard
// incomplete) is treated as stale and rebuilt.
const templateIsValid = (): boolean => {
  try {
    const out = execSync(
      `${dockerCompose} exec db psql -U defguard -d defguard_template -tAc ` +
        `"SELECT 1 FROM wireguard_network WHERE name = '_e2e_wizard_done' LIMIT 1"`,
      { stdio: 'pipe' },
    )
      .toString()
      .trim();
    return out === '1';
  } catch {
    // Template DB missing or query failed - treat as invalid.
    return false;
  }
};

const completeWizard = async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();

  // Login as admin
  await page.goto(testsConfig.BASE_URL);
  await loginBasic(page, defaultUserAdmin);

  // Complete the wizard by creating a dummy network.
  // This marks the wizard as done so /me stops redirecting to /admin/wizard.
  await page.getByTestId('setup-network').click();
  const navNext = page.getByTestId('wizard-next');
  await page.getByTestId('setup-option-manual').click();
  await navNext.click();

  await page.getByTestId('field-name').fill('_e2e_wizard_done');
  await page.getByTestId('field-address').fill('10.255.255.1/24');
  await page.getByTestId('field-endpoint').fill('127.0.0.1');
  await page.getByTestId('field-port').fill('55555');

  const responsePromise = page.waitForResponse('**/network');
  await navNext.click();
  const response = await responsePromise;
  if (response.status() !== 201) {
    throw new Error(`Wizard network creation failed: ${response.status()}`);
  }

  await browser.close();
  console.log('Wizard completed - dummy network created.');
};

const globalSetup = async () => {
  dockerUp();
  if (!templateIsValid()) {
    resetToFreshDb();
    await completeWizard();
    dockerCreateTemplate();
  } else {
    console.log('Valid template already exists, skipping wizard.');
  }
};

export default globalSetup;
