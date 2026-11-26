import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { NetworkForm } from '../types';
import { createGroup } from '../utils/controllers/groups';
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

  test('Locations table shows empty state, columns and row actions', async ({
    page,
    browser,
  }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.locations);
    await expect(page.getByText('No locations found')).toBeVisible();
    await expect(page.getByTestId('add-location')).toBeVisible();

    await createRegularLocation(browser, alphaLocation);
    await page.reload();
    const header = page.locator('.table-header');
    for (const column of [
      'Name',
      'Gateway status',
      'Gateway IP',
      'Firewall',
      'VPN network',
      'MFA',
      'Location type',
      'FWMark',
      'MTU',
      'Allowed groups',
    ]) {
      await expect(header.getByText(column, { exact: true })).toBeVisible();
    }
    const row = page.locator('.virtual-row').filter({ hasText: alphaLocation.name });
    await expect(row).toContainText('None connected');
    await expect(row).toContainText('Regular');
    await expect(row).toContainText('All allowed');
    await expect(page.getByPlaceholder('Search')).toBeVisible();

    await row.locator('.edit-cell .icon-button').click();
    const menu = page.locator('.menu');
    for (const action of ['Edit', 'Gateway setup', 'Delete']) {
      await expect(menu.getByText(action, { exact: true })).toBeVisible();
    }
  });

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

  test('Edit location network settings and access', async ({ page, browser }) => {
    const group = 'vpn-users';
    await waitForBase(page);
    await createGroup(browser, group);
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
    await page.getByTestId('field-dns').fill('1.1.1.1');
    await page.getByTestId('field-keepalive_interval').fill('30');
    await page.getByTestId('field-peer_disconnect_threshold').fill('200');
    await page.getByTestId('field-mtu').fill('1380');
    await page.locator('.toggle', { hasText: 'All groups have access' }).click();
    await page.getByRole('button', { name: 'Edit groups' }).click();
    const selectionModal = page.locator('#selection-modal');
    await selectionModal
      .locator('.item', { hasText: group })
      .locator('.checkbox')
      .click();
    await selectionModal.getByRole('button', { name: 'Submit', exact: true }).click();
    await page.getByTestId('save-location').click();

    const modal = page.locator('#action-modal');
    await expect(modal).toContainText('Save location changes');
    for (const field of ['Gateway MTU', 'All groups have access', 'Location Access']) {
      await expect(modal).toContainText(field);
    }
    for (const field of ['DNS', 'Keep alive', 'Client disconnect threshold']) {
      await expect(modal).not.toContainText(field);
    }
    const saveResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'PUT' && /\/network\/\d+$/.test(response.url()),
    );
    await modal.getByRole('button', { name: 'Save changes anyway' }).click();
    expect((await saveResponse).ok()).toBeTruthy();

    await page.goto(editUrl);
    await expect(page.getByTestId('field-dns')).toHaveValue('1.1.1.1');
    await expect(page.getByTestId('field-keepalive_interval')).toHaveValue('30');
    await expect(page.getByTestId('field-peer_disconnect_threshold')).toHaveValue('200');
    await expect(page.getByTestId('field-mtu')).toHaveValue('1380');

    await page.goto(routes.base + routes.locations);
    const row = page.locator('.virtual-row').filter({ hasText: alphaLocation.name });
    await expect(row).toContainText('1380');
    await expect(row).toContainText(group);
    await expect(row).not.toContainText('All allowed');
  });

  test.describe('Location firewall', () => {
    test.skip(!process.env.DEFGUARD_LICENSE_KEY, 'Requires DEFGUARD_LICENSE_KEY.');

    test('Firewall option is saved and shown in tables and rule form', async ({
      page,
      browser,
    }) => {
      await waitForBase(page);
      await createRegularLocation(browser, alphaLocation);
      await loginBasic(page, defaultUserAdmin);
      await page.goto(routes.base + routes.locations);
      const row = page.locator('.virtual-row').filter({ hasText: alphaLocation.name });
      const firewallIcon = row.locator('.cell-with-check-icons .icon').first();
      await expect(firewallIcon).toHaveAttribute('data-kind', 'disabled');

      await row.locator('.edit-cell .icon-button').click();
      await page.locator('.menu').getByText('Edit', { exact: true }).click();
      await page.waitForURL('**/edit');
      const editUrl = page.url();
      await expect(page.getByTestId('field-firewall-disabled')).toHaveAttribute(
        'data-active',
        'true',
      );

      for (const option of ['deny', 'allow', 'disabled']) {
        await page.goto(editUrl);
        await page.getByTestId(`field-firewall-${option}`).click();
        const saveResponse = page.waitForResponse(
          (response) =>
            response.request().method() === 'PUT' &&
            /\/network\/\d+$/.test(response.url()),
        );
        await page.getByTestId('save-location').click();
        expect((await saveResponse).ok()).toBeTruthy();

        await page.goto(editUrl);
        await expect(page.getByTestId(`field-firewall-${option}`)).toHaveAttribute(
          'data-active',
          'true',
        );
        await page.goto(routes.base + routes.locations);
        await expect(firewallIcon).toHaveAttribute(
          'data-kind',
          option === 'disabled' ? 'disabled' : 'check-filled',
        );
      }
    });

    test('Rule form shows the location firewall option', async ({ page, browser }) => {
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
      await page.getByTestId('field-firewall-deny').click();
      const saveResponse = page.waitForResponse(
        (response) =>
          response.request().method() === 'PUT' && /\/network\/\d+$/.test(response.url()),
      );
      await page.getByTestId('save-location').click();
      expect((await saveResponse).ok()).toBeTruthy();

      await page.goto(routes.base + routes.firewall.rules);
      await page.getByRole('button', { name: 'Create new rule' }).click();
      await page.locator('.toggle', { hasText: 'Include all locations' }).click();
      await page.getByRole('button', { name: 'Edit locations' }).click();
      await page
        .locator('#selection-modal .location-selection-item', {
          hasText: alphaLocation.name,
        })
        .locator('.icon')
        .hover();
      await expect(
        page.getByText(
          'Location access denied by default - network traffic not explicitly ' +
            'defined by the rules will be blocked.',
        ),
      ).toBeVisible();
    });
  });

  test.describe('Group-level MTU', () => {
    test.skip(!process.env.DEFGUARD_LICENSE_KEY, 'Requires DEFGUARD_LICENSE_KEY.');

    test('Group-level MTU overrides are saved', async ({ page, browser }) => {
      const firstGroup = 'mtu-first';
      const secondGroup = 'mtu-second';
      await waitForBase(page);
      await createGroup(browser, firstGroup);
      await createGroup(browser, secondGroup);
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
      await page.getByText('Assign custom MTU value for this location').click();
      await page.getByTestId('field-client_mtu').fill('1400');

      const addOverride = page.getByRole('button', { name: 'Add group-level MTU' });
      const modal = page.locator('#group-mtu-modal');
      const cards = page.locator('.group-mtu-card');
      for (const [mtu, groups] of [
        ['1380', [firstGroup]],
        ['1300', [secondGroup, 'admin']],
      ] as const) {
        await addOverride.click();
        await modal.locator('input').fill(mtu);
        await modal.getByRole('button', { name: 'Continue', exact: true }).click();
        for (const group of groups) {
          await modal.locator('.item', { hasText: group }).locator('.checkbox').click();
        }
        await modal.getByRole('button', { name: 'Submit', exact: true }).click();
        await modal.waitFor({ state: 'hidden' });
      }
      await expect(cards).toHaveCount(2);
      await expect(cards.filter({ hasText: '1380' })).toContainText(firstGroup);
      await expect(cards.filter({ hasText: '1300' })).toContainText(secondGroup);

      await addOverride.click();
      await expect(modal).toContainText('Every group already has a group-level MTU');
      await modal.getByRole('button', { name: 'Close', exact: true }).click();
      await modal.waitFor({ state: 'hidden' });

      await cards
        .filter({ hasText: '1380' })
        .getByRole('button', { name: 'Remove group-level MTU' })
        .click();
      await expect(cards).toHaveCount(1);
      await addOverride.click();
      await modal.locator('input').fill('1350');
      await modal.getByRole('button', { name: 'Continue', exact: true }).click();
      await expect(modal.locator('.item', { hasText: firstGroup })).toBeVisible();
      await modal.getByRole('button', { name: 'Cancel', exact: true }).click();
      await modal.waitFor({ state: 'hidden' });

      const saveResponse = page.waitForResponse(
        (response) =>
          response.request().method() === 'PUT' && /\/network\/\d+$/.test(response.url()),
      );
      await page.getByTestId('save-location').click();
      expect((await saveResponse).ok()).toBeTruthy();

      await page.goto(editUrl);
      await expect(page.getByTestId('field-client_mtu')).toHaveValue('1400');
      await expect(cards).toHaveCount(1);
      await expect(cards.first()).toContainText('1300');
      await expect(cards.first()).toContainText(secondGroup);
      await expect(cards.first()).not.toContainText(firstGroup);
    });
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
