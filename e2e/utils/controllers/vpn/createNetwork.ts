import { Browser, expect, request } from '@playwright/test';

import { defaultUserAdmin, routes, testsConfig } from '../../../config';
import { NetworkForm } from '../../../types';
import { waitForBase } from '../../waitForBase';
import { loginBasic } from '../login';

export const createNetwork = async (browser: Browser, network: NetworkForm) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.admin.wizard);

  // Wizard may redirect to overview if a network already exists (e.g. template
  // has a completed wizard).  Try the wizard flow; if 'setup-network' is not
  // present, fall back to creating the network via the API.
  const setupButton = page.getByTestId('setup-network');
  try {
    await setupButton.waitFor({ state: 'visible', timeout: 5000 });
  } catch {
    // Wizard already completed - create network via API instead.
    const apiCtx = await request.newContext({ baseURL: testsConfig.BASE_URL });
    const authRes = await apiCtx.post('/api/v1/auth', {
      data: {
        username: defaultUserAdmin.username,
        password: defaultUserAdmin.password,
      },
    });
    if (!authRes.ok()) throw new Error(`Auth failed: ${authRes.status()}`);
    // Delete any existing networks (e.g. the dummy one from the template)
    // so that devices get assigned to the newly created network.
    const listRes = await apiCtx.get('/api/v1/network');
    const existing: { id: number }[] = await listRes.json();
    for (const net of existing) {
      await apiCtx.delete(`/api/v1/network/${net.id}`);
    }
    const res = await apiCtx.post('/api/v1/network', {
      data: {
        name: network.name,
        address: network.address,
        port: parseInt(network.port, 10) || 55555,
        endpoint: network.endpoint,
        allowed_ips: '',
        dns: '',
        allowed_groups: [],
        keepalive_interval: 25,
        peer_disconnect_threshold: 300,
        acl_enabled: false,
        acl_default_allow: false,
        // The wizard UI uses 'none' for the disabled mode; the API enum only
        // accepts 'disabled' | 'internal' | 'external', so normalize here.
        location_mfa_mode:
          !network.location_mfa_mode || network.location_mfa_mode === 'none'
            ? 'disabled'
            : network.location_mfa_mode,
        service_location_mode: 'disabled',
      },
    });
    if (!res.ok()) throw new Error(`Network creation failed: ${res.status()}`);
    await apiCtx.dispose();
    await context.close();
    return;
  }

  await setupButton.click();
  const navNext = page.getByTestId('wizard-next');
  await page.getByTestId('setup-option-manual').click();
  await navNext.click();

  // fill form
  for (const key of Object.keys(network).filter((key) => key !== 'location_mfa_mode')) {
    const field = page.getByTestId(`field-${key}`);
    await field.clear();
    await field.type(network[key]);
  }
  // select location MFA mode
  if (network.location_mfa_mode) {
    const mfaModeSelect = page.locator('div.location-mfa-mode-select');
    let mode: number;
    switch (network.location_mfa_mode) {
      case 'none':
        mode = 0;
        break;
      case 'internal':
        mode = 1;
        break;
      case 'external':
        mode = 2;
        break;
      default:
        mode = 0;
        break;
    }
    const mfaMode = mfaModeSelect.locator('div.location-mfa-mode').nth(mode);
    await mfaMode.click();
  }

  const responseCreateNetworkPromise = page.waitForResponse('**/network');
  await navNext.click();
  const response = await responseCreateNetworkPromise;
  expect(response.status()).toBe(201);
  await context.close();
};
