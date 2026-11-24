import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { NetworkForm } from '../types';
import { loginBasic } from '../utils/controllers/login';
import { createMfaFlow } from '../utils/controllers/mfaFlow';
import {
  createRegularLocation,
  createServiceLocation,
} from '../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

test.describe('Setup VPN (wizard) ', () => {
  test.beforeAll(() => {
    dockerRestart();
  });

  test.afterEach(() => {
    dockerRestart();
  });

  test('Wizard Regular Location', async ({ page, browser }) => {
    await waitForBase(page);
    const network: NetworkForm = {
      name: 'test regular',
      address: '10.10.10.1/24',
      endpoint: '127.0.0.1',
      port: '5055',
      allowed_ips: ['127.1.5.1'],
    };
    await createRegularLocation(browser, network);
  });

  test('Wizard Service Location', async ({ page, browser }) => {
    await waitForBase(page);
    const network: NetworkForm = {
      name: 'test service',
      address: '10.10.10.1/24',
      endpoint: '127.0.0.1',
      port: '5055',
      allowed_ips: ['127.1.5.1'],
    };
    await createServiceLocation(browser, network);
  });

  test('Wizard Regular Location with MFA flow', async ({ page, browser }) => {
    const flowTitle = 'wizard totp flow';
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await createMfaFlow(page, flowTitle, [['totp']]);
    const flowRowTestId = await page
      .locator('[data-testid^="mfa-flow-row-"]')
      .filter({ hasText: flowTitle })
      .getAttribute('data-testid');
    const flowId = Number(flowRowTestId?.replace('mfa-flow-row-', ''));

    const network: NetworkForm = {
      name: 'test mfa',
      address: '10.10.10.1/24',
      endpoint: '127.0.0.1',
      port: '5055',
      mfa_enabled: true,
      defaultMfaFlowId: flowId,
    };
    await createRegularLocation(browser, network);

    await page.goto(routes.base + routes.mfa.list);
    await expect(
      page.locator('.virtual-row').filter({ hasText: flowTitle }),
    ).toContainText(network.name);
  });
});
