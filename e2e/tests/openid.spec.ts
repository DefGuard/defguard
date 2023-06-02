import { BrowserContext, expect, Page, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { OpenIdClient, User } from '../types';
import { acceptRecovery } from '../utils/controllers/acceptRecovery';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic, loginTOTP } from '../utils/controllers/login';
import { logout } from '../utils/controllers/logout';
import { enableTOTP } from '../utils/controllers/mfa/enableTOTP';
import { CreateOpenIdClient } from '../utils/controllers/openid/createOpenIdClient';
import { dockerRestart } from '../utils/docker';
import { getPageClipboard } from '../utils/getPageClipboard';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';
import { waitForRoute } from '../utils/waitForRoute';

// FIXME containerize test client so tests can run without external testing client

test.describe.configure({
  mode: 'serial',
});

test.describe('Authorize OpenID client.', () => {
  let testUser: User;

  const client: OpenIdClient = {
    name: 'test 01',
    redirectURL: 'https://oidcdebugger.com/debug',
    scopes: ['openid'],
  };

  let page: Page;
  let context: BrowserContext;

  // Setup client and user for tests
  test.beforeAll(async ({ browser }) => {
    context = await browser.newContext();
    testUser = await createUser(context, 'testopenid');
    page = await context.newPage();
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await waitForRoute(page, routes.admin.wizard);
    await CreateOpenIdClient(page, client);
    await page.getByTestId('edit-openid-client-1').click();
    await page.getByTestId('copy-openid-client-id').click();
    const clientId = await getPageClipboard(page);
    client.clientID = clientId;
    await logout(page);
  });

  test.afterAll(() => {
    dockerRestart();
  });

  test('Authorize when session is active.', async () => {
    expect(client.clientID).toBeDefined();
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

  test('Authorize when session is not active', async () => {
    expect(client.clientID).toBeDefined();
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

  test('Authorize when session is not active and MFA is enabled', async () => {
    expect(client.clientID).toBeDefined();
    await page.goto(routes.base + routes.auth.login);
    await loginBasic(page, testUser);
    await waitForRoute(page, routes.me);
    const totp_secret = await enableTOTP(page);
    await acceptRecovery(page);
    await waitForRoute(page, routes.auth.login);
    await fillAndSubmitOpenIDDebugger(page, client);
    await waitForRoute(page, routes.auth.login);
    await loginTOTP(page, testUser, totp_secret);
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
  client: OpenIdClient
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
