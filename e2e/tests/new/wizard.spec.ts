import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { NetworkForm } from '../../types';
import {
  createRegularLocation,
  createServiceLocation,
} from '../../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';

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
});
