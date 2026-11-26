import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { Protocols } from '../types';
import {
  apiApplyRules,
  apiCreateAlias,
  apiCreateDestination,
  apiCreateRule,
} from '../utils/api/firewall';
import {
  addAddresses,
  aliasColumns,
  destinationColumns,
  fieldFold,
  formToggle,
  tableCell,
  tableRow,
} from '../utils/controllers/firewallRules';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';
import { createAlias } from '../utils/firewall';
import { waitForBase } from '../utils/waitForBase';

test.describe('Test aliases', () => {
  test.beforeEach(() => {
    dockerRestart();
  });

  test('Create alias and check content', async ({ page, browser }) => {
    const name = 'TestAlias';
    const addresses = ['1.2.3.4/24', '10.10.10.10/20', '1.2.4.2'];
    const ports = ['80', '443'];
    const protocols = [Protocols.UDP, Protocols.ICMP];
    await createAlias(browser, name, addresses, ports, protocols);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.firewall.aliases);
    const aliasRow = page.locator('.virtual-row').filter({ hasText: name });
    await expect(aliasRow).toBeVisible();
    await expect(aliasRow).toContainText(addresses.join(', '));
    await expect(aliasRow).toContainText(ports.join(', '));
    await expect(aliasRow).toContainText(Protocols.UDP);
    await expect(aliasRow).toContainText(Protocols.ICMP);
  });
});

test.describe('Firewall aliases and destinations', () => {
  test.beforeEach(async ({ page }) => {
    test.skip(!process.env.DEFGUARD_LICENSE_KEY, 'Firewall is a business feature');
    dockerRestart();
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
  });

  test('Add destination', async ({ page }) => {
    await page.goto(routes.base + routes.firewall.destinations);
    await page.getByRole('button', { name: 'Add new destination' }).click();
    await page.waitForURL('**' + routes.firewall.addDestination + '**');
    for (const label of ['Any address', 'Any port', 'Any protocol']) {
      await expect(formToggle(page, label)).toHaveAttribute('data-value', 'true');
    }
    for (const field of ['field-addresses', 'field-ports', 'field-protocols-6']) {
      await expect(fieldFold(page, field)).toHaveClass('fold folded');
    }

    await page.getByTestId('field-name').fill('web servers');
    await formToggle(page, 'Any address').click();
    await addAddresses(page, ['10.5.0.0/24', '10.6.0.1']);
    await formToggle(page, 'Any port').click();
    await page.getByTestId('field-ports').fill('80, 443');
    await formToggle(page, 'Any protocol').click();
    await page.getByTestId('field-protocols-6').click();
    await page.getByTestId('field-protocols-17').click();
    await page.getByRole('button', { name: 'Add destination' }).click();
    await expect(page.getByText('Destination created.')).toBeVisible();
    await page.waitForURL('**' + routes.firewall.destinations + '**');

    await page.reload();
    await expect(page.locator('.header-cell')).toHaveText(destinationColumns.slice(1));
    await expect(page.getByPlaceholder('Search')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Add new destination' })).toBeVisible();
    const row = tableRow(page, 'web servers');
    await expect(tableCell(row, destinationColumns, 'Destination name')).toHaveText(
      'web servers',
    );
    await expect(
      tableCell(row, destinationColumns, 'IPv4/IPv6 CIDR ranges or addresses'),
    ).toHaveText('10.5.0.0/24, 10.6.0.1');
    await expect(tableCell(row, destinationColumns, 'Ports')).toHaveText('80, 443');
    await expect(tableCell(row, destinationColumns, 'Protocols')).toHaveText('TCP, UDP');
    await expect(tableCell(row, destinationColumns, 'Used in rules')).toHaveText('');
  });

  test('Edit destination used in deployed rule', async ({ page }) => {
    const destination = await apiCreateDestination(page, 'web servers');
    const rule = await apiCreateRule(page, 'web rule', {
      destinations: [destination],
      use_manual_destination_settings: false,
    });
    await apiApplyRules(page, [rule]);

    await page.goto(routes.base + routes.firewall.destinations);
    const row = tableRow(page, 'web servers');
    await expect(tableCell(row, destinationColumns, 'Used in rules')).toHaveText(
      'web rule',
    );
    await row.locator('.icon-button').click();
    await page.getByTestId('destination-row-edit').click();
    await page.getByTestId('field-ports').fill('8443');
    await page.getByRole('button', { name: 'Save changes' }).click();
    await expect(
      page.getByText('Destination added to Pending tab and awaiting deployment.'),
    ).toBeVisible();
    await page.waitForURL('**' + routes.firewall.destinations + '**');

    await page.reload();
    await page.getByTestId('destinations-tab-deployed').click();
    await expect(tableCell(row, destinationColumns, 'Ports')).toHaveText('443');
    await page.getByTestId('destinations-tab-pending').click();
    await expect(page.getByTestId('destinations-tab-pending')).toHaveText('Pending (1)');
    await expect(
      page.locator('.nav-item', { hasText: 'Destinations' }).locator('.counter-label'),
    ).toHaveText('1');
    await expect(tableCell(row, destinationColumns, 'Ports')).toHaveText('8443');

    await page.getByRole('button', { name: 'Deploy all pending (1)' }).click();
    await expect(row).toBeHidden();
    await page.reload();
    await page.getByTestId('destinations-tab-deployed').click();
    await expect(tableCell(row, destinationColumns, 'Ports')).toHaveText('8443');
  });

  test('Edit alias used in deployed rule', async ({ page }) => {
    const alias = await apiCreateAlias(page, 'web ports');
    const rule = await apiCreateRule(page, 'web rule', { aliases: [alias] });
    await apiApplyRules(page, [rule]);

    await page.goto(routes.base + routes.firewall.aliases);
    await expect(page.locator('.header-cell')).toHaveText([...aliasColumns.slice(1), '']);
    const row = tableRow(page, 'web ports');
    await expect(tableCell(row, aliasColumns, 'Used in rules')).toHaveText('web rule');
    await row.locator('.icon-button').click();
    await page.getByTestId('alias-row-edit').click();
    await page.getByTestId('field-ports').fill('8443');
    await page.getByRole('button', { name: 'Save changes' }).click();
    await expect(
      page.getByText('Alias added to Pending tab and awaiting deployment.'),
    ).toBeVisible();
    await page.waitForURL('**' + routes.firewall.aliases + '**');

    await page.reload();
    await page.getByTestId('aliases-tab-deployed').click();
    await expect(tableCell(row, aliasColumns, 'Ports')).toHaveText('443');
    await page.getByTestId('aliases-tab-pending').click();
    await expect(page.getByTestId('aliases-tab-pending')).toHaveText('Pending (1)');
    await expect(
      page.locator('.nav-item', { hasText: 'Aliases' }).locator('.counter-label'),
    ).toHaveText('1');
    await expect(tableCell(row, aliasColumns, 'Ports')).toHaveText('8443');

    await page.getByTestId('aliases-deploy-all-pending').click();
    await expect(row).toBeHidden();
    await page.reload();
    await page.getByTestId('aliases-tab-deployed').click();
    await expect(tableCell(row, aliasColumns, 'Ports')).toHaveText('8443');
  });

  test('Add destination with address range and any port and protocol', async ({
    page,
  }) => {
    await page.goto(routes.base + routes.firewall.destinations);
    await page.getByRole('button', { name: 'Add new destination' }).click();
    await page.getByTestId('field-name').fill('range servers');
    await formToggle(page, 'Any address').click();
    await addAddresses(page, ['10.0.0.1-10.0.0.10']);
    await page.getByRole('button', { name: 'Add destination' }).click();
    await expect(page.getByText('Destination created.')).toBeVisible();

    await page.reload();
    const row = tableRow(page, 'range servers');
    await expect(
      tableCell(row, destinationColumns, 'IPv4/IPv6 CIDR ranges or addresses'),
    ).toHaveText('10.0.0.1-10.0.0.10');
    await expect(tableCell(row, destinationColumns, 'Ports')).toHaveText('Any port');
    await expect(tableCell(row, destinationColumns, 'Protocols')).toHaveText(
      'Any protocol',
    );
  });

  test('Add alias with address range', async ({ page, browser }) => {
    await createAlias(browser, 'range alias', ['10.0.0.1-10.0.0.10'], ['8080']);
    await page.goto(routes.base + routes.firewall.aliases);
    const row = tableRow(page, 'range alias');
    await expect(tableCell(row, aliasColumns, 'IPv4/IPv6 CIDR ranges')).toHaveText(
      '10.0.0.1-10.0.0.10',
    );
    await expect(tableCell(row, aliasColumns, 'Ports')).toHaveText('8080');
  });

  test('Search aliases and destinations', async ({ page }) => {
    await apiCreateAlias(page, 'web ports');
    await apiCreateAlias(page, 'db ports');
    await apiCreateDestination(page, 'web servers');
    await apiCreateDestination(page, 'db servers');

    await page.goto(routes.base + routes.firewall.aliases);
    await expect(tableRow(page, 'db ports')).toBeVisible();
    await page.getByPlaceholder('Search').fill('web');
    await expect(tableRow(page, 'web ports')).toBeVisible();
    await expect(tableRow(page, 'db ports')).toHaveCount(0);

    await page.goto(routes.base + routes.firewall.destinations);
    await expect(tableRow(page, 'db servers')).toBeVisible();
    await page.getByPlaceholder('Search').fill('web');
    await expect(tableRow(page, 'web servers')).toBeVisible();
    await expect(tableRow(page, 'db servers')).toHaveCount(0);
  });
});
