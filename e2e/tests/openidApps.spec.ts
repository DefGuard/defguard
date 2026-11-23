import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { OpenIdClient } from '../types';
import { loginBasic } from '../utils/controllers/login';
import { CreateOpenIdClient } from '../utils/controllers/openid/createOpenIdClient';
import { dockerRestart } from '../utils/docker';
import { getPageClipboard } from '../utils/getPageClipboard';
import { waitForBase } from '../utils/waitForBase';

const testClient: OpenIdClient = {
  name: 'test app',
  redirectURL: ['https://app.test/callback'],
  scopes: ['openid', 'email'],
};

test.describe('OpenID applications', () => {
  test.beforeEach(() => dockerRestart());

  test('Add OpenID application and list it', async ({ page, browser }) => {
    const secondClient: OpenIdClient = {
      name: 'second app',
      redirectURL: ['https://second.test/callback'],
      scopes: ['openid'],
    };
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.openid_apps);
    await expect(page.getByText("You don't have any OpenID applications.")).toBeVisible();

    await page.getByTestId('add-new-app').click();
    const modal = page.locator('#ce-openid-client-modal');
    await modal.waitFor({ state: 'visible' });
    await modal.getByTestId('field-name').fill(testClient.name);
    await modal.getByTestId('field-redirect_uri[0]').fill(testClient.redirectURL[0]);
    await modal.getByTestId('add-url').click();
    await modal.getByTestId('field-redirect_uri[1]').fill('https://app.test/second');
    for (const scope of testClient.scopes) {
      await modal.getByTestId(`field-scope-${scope}`).click();
    }
    await modal.getByTestId('save-settings').click();
    await modal.waitFor({ state: 'hidden' });

    await CreateOpenIdClient(browser, secondClient);
    await page.reload();
    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(2);
    await expect(rows.filter({ hasText: testClient.name })).toContainText('Enabled');
    await expect(rows.filter({ hasText: secondClient.name })).toContainText('Enabled');

    await page
      .locator('.virtual-row')
      .filter({ hasText: testClient.name })
      .locator('.icon-button')
      .click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    await modal.waitFor({ state: 'visible' });
    await expect(modal.getByTestId('field-name')).toHaveValue(testClient.name);
    await expect(modal.getByTestId('field-redirect_uri[0]')).toHaveValue(
      testClient.redirectURL[0],
    );
    await expect(modal.getByTestId('field-redirect_uri[1]')).toHaveValue(
      'https://app.test/second',
    );
    await expect(modal.getByTestId('field-scope-openid')).toHaveAttribute(
      'data-active',
      'true',
    );
    await expect(modal.getByTestId('field-scope-email')).toHaveAttribute(
      'data-active',
      'true',
    );
    await expect(modal.getByTestId('field-scope-phone')).toHaveAttribute(
      'data-active',
      'false',
    );
  });

  test('Edit OpenID application', async ({ page, browser }) => {
    const newName = 'edited app';
    const newUrl = 'https://edited.test/callback';
    await waitForBase(page);
    await CreateOpenIdClient(browser, testClient);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.openid_apps);

    await page
      .locator('.virtual-row')
      .filter({ hasText: testClient.name })
      .locator('.icon-button')
      .click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    const modal = page.locator('#ce-openid-client-modal');
    await modal.waitFor({ state: 'visible' });
    await modal.getByTestId('field-name').fill(newName);
    await modal.getByTestId('field-redirect_uri[0]').fill(newUrl);
    await modal.getByTestId('field-scope-profile').click();
    await modal.getByRole('button', { name: 'Save changes', exact: true }).click();
    await modal.waitFor({ state: 'hidden' });

    await page.reload();
    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(1);
    await expect(rows.filter({ hasText: newName })).toBeVisible();
    await expect(rows.filter({ hasText: testClient.name })).toHaveCount(0);

    await page
      .locator('.virtual-row')
      .filter({ hasText: newName })
      .locator('.icon-button')
      .click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    await modal.waitFor({ state: 'visible' });
    await expect(modal.getByTestId('field-name')).toHaveValue(newName);
    await expect(modal.getByTestId('field-redirect_uri[0]')).toHaveValue(newUrl);
    await expect(modal.getByTestId('field-scope-profile')).toHaveAttribute(
      'data-active',
      'true',
    );
  });

  test('Disable and enable OpenID application', async ({ page, browser }) => {
    await waitForBase(page);
    await CreateOpenIdClient(browser, testClient);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.openid_apps);
    const row = page.locator('.virtual-row').filter({ hasText: testClient.name });
    await expect(row).toContainText('Enabled');

    await page
      .locator('.virtual-row')
      .filter({ hasText: testClient.name })
      .locator('.icon-button')
      .click();
    await page.locator('.menu').getByText('Disable', { exact: true }).click();
    await expect(row).toContainText('Disabled');
    await page.reload();
    await expect(row).toContainText('Disabled');

    await page
      .locator('.virtual-row')
      .filter({ hasText: testClient.name })
      .locator('.icon-button')
      .click();
    await page.locator('.menu').getByText('Enable', { exact: true }).click();
    await expect(row).toContainText('Enabled');
    await page.reload();
    await expect(row).toContainText('Enabled');
  });

  test('Delete OpenID application', async ({ page, browser }) => {
    await waitForBase(page);
    await CreateOpenIdClient(browser, testClient);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.openid_apps);

    await page
      .locator('.virtual-row')
      .filter({ hasText: testClient.name })
      .locator('.icon-button')
      .click();
    await page.locator('.menu').getByText('Delete', { exact: true }).click();
    await page.getByTestId('confirm-action-submit').click();

    await expect(page.getByText("You don't have any OpenID applications.")).toBeVisible();
    await page.reload();
    await expect(page.getByText("You don't have any OpenID applications.")).toBeVisible();
    await expect(page.locator('.virtual-row')).toHaveCount(0);
  });

  test('Copy client ID and secret', async ({ page, browser }) => {
    await waitForBase(page);
    await CreateOpenIdClient(browser, testClient);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.openid_apps);

    await page
      .locator('.virtual-row')
      .filter({ hasText: testClient.name })
      .locator('.icon-button')
      .click();
    await page.getByTestId('copy-id').click();
    const clientId = await getPageClipboard(page);

    await page
      .locator('.virtual-row')
      .filter({ hasText: testClient.name })
      .locator('.icon-button')
      .click();
    await page.getByTestId('copy-secret').click();
    const clientSecret = await getPageClipboard(page);

    expect(clientId).toMatch(/^\S{8,}$/);
    expect(clientSecret).toMatch(/^\S{8,}$/);
    expect(clientSecret).not.toBe(clientId);
  });
});
