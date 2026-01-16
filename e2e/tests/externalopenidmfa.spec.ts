import { expect, test } from '@playwright/test';

import { defaultUserAdmin, testsConfig, testUserTemplate } from '../config';
import { NetworkForm, OpenIdClient, User } from '../types';
import { apiCreateUser } from '../utils/api/users';
import { loginBasic } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { copyOpenIdClientIdAndSecret } from '../utils/controllers/openid/copyClientId';
import { createExternalProvider } from '../utils/controllers/openid/createExternalProvider';
import { CreateOpenIdClient } from '../utils/controllers/openid/createOpenIdClient';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';

test.describe('External OIDC.', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };

  const client: OpenIdClient = {
    name: 'test 01',
    redirectURL: [
      `${testsConfig.BASE_URL}/auth/callback`,
      `${testsConfig.ENROLLMENT_URL}/openid/callback`,
    ],
    scopes: ['openid', 'profile', 'email'],
  };

  const testNetwork: NetworkForm = {
    name: 'test network',
    address: '10.10.10.1/24',
    allowed_ips: ['1.2.3.4'],
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
    await createRegularLocation(browser, testNetwork);
    await context.close();
  });

  // TODO: Finish when https://github.com/DefGuard/defguard/issues/1817 is resolved
  // test('Login through external oidc.', async ({ page }) => {
  //   expect(client.clientID).toBeDefined();
  //   expect(client.clientSecret).toBeDefined();
  //   await waitForBase(page);
  //   const oidcLoginButton = await page.locator('.oidc-button');
  //   expect(oidcLoginButton).not.toBeNull();
  //   expect(await oidcLoginButton.textContent()).toBe(`Sign in with ${client.name}`);
  //   await oidcLoginButton.click();
  //   await page.getByTestId('login-form-username').fill(testUser.username);
  //   await page.getByTestId('login-form-password').fill(testUser.password);
  //   await page.getByTestId('login-form-submit').click();
  //   await page.getByTestId('openid-allow').click();
  //   await waitForRoute(page, routes.me);
  //   const authorizedApps = await page
  //     .getByTestId('authorized-apps')
  //     .locator('div')
  //     .textContent();
  //   expect(authorizedApps).toContain(client.name);
  // });

  test('Sign in with external SSO', async ({ page }) => {
    await waitForBase(page);
    await page.goto(testsConfig.ENROLLMENT_URL);
    await waitForPromise(2000);
    await page.getByTestId('start-enrollment').click();
    await page.locator('.oidc-button-link').click();
    await page.getByTestId('field-username').fill(defaultUserAdmin.username);
    await page.getByTestId('field-password').fill(defaultUserAdmin.password);
    await page.getByTestId('sign-in').click();
    await page.getByTestId('accept-openid').click();
    await page.getByTestId('page-nav-next').click();
    await page.getByTestId('modal-confirm-download-submit').click();

    const setup_desktop = await page.locator('#setup-desktop');
    await setup_desktop.locator('.fold-button').click();

    const token = await page
      .locator('.copy-field')
      .filter({ hasText: 'Token' })
      .locator('.track p')
      .textContent();
    expect(token).toBeDefined();
  });
});
