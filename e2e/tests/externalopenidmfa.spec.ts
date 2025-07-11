import { expect, test } from '@playwright/test';

import { defaultUserAdmin, testsConfig, testUserTemplate } from '../config';
import { NetworkForm, OpenIdClient, User } from '../types';
import { apiCreateUser, apiGetUserProfile } from '../utils/api/users';
import { loginBasic } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { copyOpenIdClientIdAndSecret } from '../utils/controllers/openid/copyClientId';
import { createExternalProvider } from '../utils/controllers/openid/createExternalProvider';
import { CreateOpenIdClient } from '../utils/controllers/openid/createOpenIdClient';
import { createDevice } from '../utils/controllers/vpn/createDevice';
import { createNetwork } from '../utils/controllers/vpn/createNetwork';
import { dockerDown, dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';

test.describe('External OIDC.', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };

  const client: OpenIdClient = {
    name: 'test 01',
    redirectURL: ['http://localhost:8080/openid/mfa/callback'],
    scopes: ['openid', 'profile', 'email'],
    use_external_openid_mfa: true,
  };

  const testNetwork: NetworkForm = {
    name: 'test network',
    address: '10.10.10.1/24',
    endpoint: '127.0.0.1',
    port: '5055',
  };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    await CreateOpenIdClient(browser, client);
    [client.clientID, client.clientSecret] = await copyOpenIdClientIdAndSecret(
      browser,
      client.name,
    );
    const context = await browser.newContext();
    const page = await context.newPage();
    await createExternalProvider(browser, client);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUser(page, testUser);
    await logout(page);
    await createNetwork(browser, testNetwork);
    await context.close();
  });

  test.afterAll(() => {
    dockerDown();
  });

  test('Complete client MFA through external OpenID', async ({ page, browser }) => {
    await waitForBase(page);
    const mfaStartUrl = `${testsConfig.ENROLLMENT_URL}/api/v1/client-mfa/start`;
    await createDevice(browser, testUser, {
      name: 'test',
    });
    await loginBasic(page, testUser);
    const testUserProfile = await apiGetUserProfile(page, testUser.username);
    expect(testUserProfile.devices.length).toBe(1);
    const createdDevice = testUserProfile.devices[0];
    const pubkey = createdDevice.wireguard_pubkey;
    const data = {
      method: 2,
      pubkey: pubkey,
      location_id: 1,
    };
    const response = await page.request.post(mfaStartUrl, {
      data: data,
    });
    expect(response.ok()).toBeTruthy();
    const { token } = await response.json();
    expect(token).toBeDefined();
    expect(token.length).toBeGreaterThan(0);

    const preconditionResponse = await page.request.post(
      testsConfig.ENROLLMENT_URL + '/api/v1/client-mfa/finish',
      {
        data: {
          token: token,
        },
      },
    );
    expect(preconditionResponse.status()).toBe(428);

    const url = testsConfig.ENROLLMENT_URL + '/openid/mfa' + `?token=${token}`;
    await page.goto(url);
    await waitForPromise(2000);
    await page.getByTestId('openid-allow').click();
    await waitForPromise(2000);

    const finish = testsConfig.ENROLLMENT_URL + '/api/v1/client-mfa/finish';
    const finishResponse = await page.request.post(finish, {
      data: {
        token: token,
      },
    });
    expect(finishResponse.ok()).toBeTruthy();
    const finishData = await finishResponse.json();
    expect(finishData.preshared_key).toBeDefined();
    expect(finishData.preshared_key.length).toBeGreaterThan(0);
  });
});
