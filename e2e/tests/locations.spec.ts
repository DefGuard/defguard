import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { NetworkForm } from '../types';
import { loginBasic } from '../utils/controllers/login';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const alphaLocation: NetworkForm = {
  name: 'alpha location',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  port: '5055',
};

const bravoLocation: NetworkForm = {
  name: 'bravo location',
  address: '10.20.20.1/24',
  endpoint: '127.0.0.2',
  port: '5056',
};

test.describe('Locations', () => {
  test.beforeEach(() => dockerRestart());

  test('List and sort locations', async ({ page, browser }) => {
    await waitForBase(page);
    await createRegularLocation(browser, bravoLocation);
    await createRegularLocation(browser, alphaLocation);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.locations);

    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(2);
    for (const [index, location] of [alphaLocation, bravoLocation].entries()) {
      await expect(rows.nth(index)).toContainText(location.name);
      await expect(rows.nth(index)).toContainText(location.endpoint);
      await expect(rows.nth(index)).toContainText(location.address);
    }

    await page.getByText('Name', { exact: true }).click();
    for (const [index, location] of [bravoLocation, alphaLocation].entries()) {
      await expect(rows.nth(index)).toContainText(location.name);
      await expect(rows.nth(index)).toContainText(location.endpoint);
      await expect(rows.nth(index)).toContainText(location.address);
    }
  });

  test('Search locations', async ({ page, browser }) => {
    await waitForBase(page);
    await createRegularLocation(browser, alphaLocation);
    await createRegularLocation(browser, bravoLocation);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.locations);

    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(2);
    const search = page.getByPlaceholder('Search');
    await search.fill('bravo');
    await expect(rows).toHaveCount(1);
    await expect(rows.first()).toContainText(bravoLocation.name);

    await search.fill('missing');
    await expect(page.getByText('Nothing is found.')).toBeVisible();
    await expect(rows).toHaveCount(0);

    await search.fill('');
    await expect(rows).toHaveCount(2);
  });

  test('Edit location name without confirmation', async ({ page, browser }) => {
    const newName = 'renamed location';
    await waitForBase(page);
    await createRegularLocation(browser, alphaLocation);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.locations);

    await page
      .locator('.virtual-row')
      .filter({ hasText: alphaLocation.name })
      .locator('.edit-cell .icon-button')
      .click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    await page.waitForURL('**/edit');
    await page.getByTestId('field-name').fill(newName);
    const saveResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'PUT' && /\/network\/\d+$/.test(response.url()),
    );
    await page.getByTestId('save-location').click();
    expect((await saveResponse).ok()).toBeTruthy();
    await expect(page.locator('#action-modal')).not.toBeVisible();

    await page.goto(routes.base + routes.locations);
    const rows = page.locator('.virtual-row');
    await expect(rows).toHaveCount(1);
    await expect(rows.first()).toContainText(newName);
  });

  test('Edit location port asks to save location changes', async ({ page, browser }) => {
    const newPort = '5099';
    await waitForBase(page);
    await createRegularLocation(browser, alphaLocation);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.locations);

    await page
      .locator('.virtual-row')
      .filter({ hasText: alphaLocation.name })
      .locator('.edit-cell .icon-button')
      .click();
    await page.locator('.menu').getByText('Edit', { exact: true }).click();
    await page.waitForURL('**/edit');
    const editUrl = page.url();
    await page.getByTestId('field-port').fill(newPort);
    await page.getByTestId('save-location').click();

    const modal = page.locator('#action-modal');
    await expect(modal).toContainText('Save location changes');
    await expect(modal).toContainText('VPN WireGuard® Port');
    await modal.getByTestId('confirm-action-cancel').click();
    await modal.waitFor({ state: 'hidden' });
    await page.reload();
    await expect(page.getByTestId('field-port')).toHaveValue(alphaLocation.port);

    await page.getByTestId('field-port').fill(newPort);
    await page.getByTestId('save-location').click();
    const saveResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'PUT' && /\/network\/\d+$/.test(response.url()),
    );
    await modal.getByRole('button', { name: 'Save changes anyway' }).click();
    expect((await saveResponse).ok()).toBeTruthy();

    await page.goto(editUrl);
    await expect(page.getByTestId('field-port')).toHaveValue(newPort);
  });

  test('Delete location', async ({ page, browser }) => {
    await waitForBase(page);
    await createRegularLocation(browser, alphaLocation);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.locations);

    await page
      .locator('.virtual-row')
      .filter({ hasText: alphaLocation.name })
      .locator('.edit-cell .icon-button')
      .click();
    await page.locator('.menu').getByText('Delete', { exact: true }).click();
    await expect(page.locator('#action-modal')).toContainText(alphaLocation.name);
    await page.getByTestId('confirm-action-submit').click();

    await expect(page.getByText('No locations found')).toBeVisible();
    await page.reload();
    await expect(page.getByText('No locations found')).toBeVisible();
    await expect(page.locator('.virtual-row')).toHaveCount(0);
  });
});
