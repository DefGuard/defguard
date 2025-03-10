import { expect, Page, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { OpenIdClient, User } from '../types';
import { apiCreateUser } from '../utils/api/users';
import { loginBasic, loginTOTP } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { enableTOTP } from '../utils/controllers/mfa/enableTOTP';
import { copyOpenIdClientId } from '../utils/controllers/openid/copyClientId';
import { CreateOpenIdClient } from '../utils/controllers/openid/createOpenIdClient';
import { dockerDown, dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';
import { waitForRoute } from '../utils/waitForRoute';

// FIXME containerize test client so tests can run without external testing client

test.describe('Authorize OpenID client.', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };

  const client: OpenIdClient = {
    name: 'test 01',
    redirectURL: ['https://oidcdebugger.com/debug'],
    scopes: ['openid'],
  };

  // Setup client and user for tests
  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    await CreateOpenIdClient(browser, client);
    client.clientID = await copyOpenIdClientId(browser, 1);
    const context = await browser.newContext();
    const page = await context.newPage();
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUser(page, testUser);
    context.close();
  });

  test.afterAll(() => {
    dockerDown();
  });

  test('Authorize when session is active.', async ({ page }) => {
    expect(client.clientID).toBeDefined();
    await waitForBase(page);
    await loginBasic(page, testUser);
    await fillAndSubmitOpenIDDebugger(page, client);
    await page.waitForURL(routes.base + routes.consent + '**');
    await page.getByTestId('openid-allow').click();
    await page.waitForURL('https://oidcdebugger.com/**');
    await waitForPromise(2000);
    const headerMessage = await page
      .locator('.debug__callback-header')
      .locator('h1')
      .textContent();
    expect(headerMessage?.replace(' ', '')).toBe('Success!');
    await page.goto(routes.base + routes.me, {
      waitUntil: 'networkidle',
    });
    await waitForRoute(page, routes.me);
    await page.getByTestId('authorized-apps').getByRole('button').click();
    await logout(page);
  });

  test('Authorize when session is not active', async ({ page }) => {
    expect(client.clientID).toBeDefined();
    await waitForBase(page);
    await fillAndSubmitOpenIDDebugger(page, client);
    await waitForRoute(page, routes.auth.login);
    await loginBasic(page, testUser);
    await page.waitForURL(routes.base + routes.consent + '**');
    await page.getByTestId('openid-allow').click();
    await page.waitForURL('https://oidcdebugger.com/**');
    await waitForPromise(2000);
    const headerMessage = await page
      .locator('.debug__callback-header')
      .locator('h1')
      .textContent();
    expect(headerMessage?.replace(' ', '')).toBe('Success!');
    await page.goto(routes.base + routes.me, {
      waitUntil: 'networkidle',
    });
    await waitForRoute(page, routes.me);
    await page.getByTestId('authorized-apps').getByRole('button').click();
    await logout(page);
  });

  test('Authorize when session is not active and MFA is enabled', async ({
    page,
    browser,
  }) => {
    expect(client.clientID).toBeDefined();
    const { secret } = await enableTOTP(browser, testUser);
    await waitForBase(page);
    await fillAndSubmitOpenIDDebugger(page, client);
    await loginTOTP(page, testUser, secret);
    await page.waitForURL(routes.base + routes.consent + '**');
    await page.getByTestId('openid-allow').click();
    await page.waitForURL('https://oidcdebugger.com/**');
    await waitForPromise(2000);
    const headerMessage = await page
      .locator('.debug__callback-header')
      .locator('h1')
      .textContent();
    expect(headerMessage?.replace(' ', '')).toBe('Success!');
    await page.goto(routes.base + routes.me, {
      waitUntil: 'networkidle',
    });
    await waitForRoute(page, routes.me);
    await page.getByTestId('authorized-apps').getByRole('button').click();
    await logout(page);
  });
});

const fillAndSubmitOpenIDDebugger = async (
  page: Page,
  client: OpenIdClient,
): Promise<void> => {
  await page.goto('https://oidcdebugger.com/');
  const authorizeUriInput = page.locator('#authorizeUri');
  await authorizeUriInput.clear();
  await authorizeUriInput.type(routes.base + routes.authorize);
  const clientIdInput = page.locator('#clientId');
  await clientIdInput.clear();
  await clientIdInput.type(client.clientID as string);
  await page.locator('.debug__form-submit').click();
};
