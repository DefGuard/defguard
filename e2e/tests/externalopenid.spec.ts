import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testsConfig, testUserTemplate } from '../config';
import { NetworkForm, OpenIdClient, User } from '../types';
import { apiCreateUser } from '../utils/api/users';
import { loginBasic } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { copyOpenIdClientIdAndSecret } from '../utils/controllers/openid/copyClientId';
import { createExternalProvider } from '../utils/controllers/openid/createExternalProvider';
import { CreateOpenIdClient } from '../utils/controllers/openid/createOpenIdClient';
import { createNetwork } from '../utils/controllers/vpn/createNetwork';
import { dockerDown, dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';
import { waitForRoute } from '../utils/waitForRoute';

test.describe('External OIDC.', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };

  const client: OpenIdClient = {
    name: 'test 01',
    redirectURL: [
      'http://localhost:8000/auth/callback',
      'http://localhost:8080/openid/callback',
    ],
    scopes: ['openid', 'profile', 'email'],
    use_external_openid_mfa: false,
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

  test('Login through external oidc.', async ({ page }) => {
    expect(client.clientID).toBeDefined();
    expect(client.clientSecret).toBeDefined();
    await waitForBase(page);
    const oidcLoginButton = await page.getByTestId('login-oidc');
    expect(oidcLoginButton).not.toBeNull();
    expect(await oidcLoginButton.textContent()).toBe(`Sign in with ${client.name}`);
    await oidcLoginButton.click();
    await page.getByTestId('login-form-username').fill(testUser.username);
    await page.getByTestId('login-form-password').fill(testUser.password);
    await page.getByTestId('login-form-submit').click();
    await page.getByTestId('openid-allow').click();
    await waitForRoute(page, routes.me);
    const authorizedApps = await page
      .getByTestId('authorized-apps')
      .locator('div')
      .textContent();
    expect(authorizedApps).toContain(client.name);
  });

  test('Complete enrollment through external OIDC', async ({ page }) => {
    await waitForBase(page);
    await page.goto(testsConfig.ENROLLMENT_URL);
    await waitForPromise(2000);
    await page.getByTestId('select-enrollment').click();
    await page.getByTestId('login-oidc').click();
    await page.getByTestId('login-form-username').fill(testUser.username);
    await page.getByTestId('login-form-password').fill(testUser.password);
    await page.getByTestId('login-form-submit').click();
    await page.getByTestId('openid-allow').click();
    const instanceUrlBox = page
      .locator('div')
      .filter({ hasText: /^Instance URL$/ })
      .getByRole('textbox');

    expect(await instanceUrlBox.inputValue()).toBe('http://localhost:8080/');
    const instanceTokenBox = page
      .locator('div')
      .filter({ hasText: /^Token$/ })
      .getByRole('textbox');
    expect((await instanceTokenBox.inputValue()).length).toBeGreaterThan(1);
  });
});
