import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { NetworkForm } from '../types';
import {
  apiApplyRules,
  apiCreateAlias,
  apiCreateDestination,
  apiCreateRule,
} from '../utils/api/firewall';
import { createUser } from '../utils/controllers/createUser';
import {
  addAddresses,
  fieldFold,
  formToggle,
  openNewRuleForm,
  openPendingRule,
  openRuleEdit,
  permissionChips,
  restriction,
  ruleCell,
  selectInModal,
  submitRule,
  tableRow,
} from '../utils/controllers/firewallRules';
import { createGroup } from '../utils/controllers/groups';
import { loginBasic } from '../utils/controllers/login';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import { startNetworkDeviceEnrollment } from '../utils/controllers/vpn/createNetworkDevice';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const firstLocation: NetworkForm = {
  name: 'office',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  port: '5055',
};

const secondLocation: NetworkForm = {
  name: 'datacenter',
  address: '10.20.20.1/24',
  endpoint: '127.0.0.1',
  port: '5056',
};

const testUser = { ...testUserTemplate, username: 'ruleuser' };

test.describe('Firewall rules', () => {
  test.beforeEach(async ({ page }) => {
    test.skip(!process.env.DEFGUARD_LICENSE_KEY, 'Firewall is a business feature');
    dockerRestart();
    await waitForBase(page);
  });

  test('Add rule for all locations, then deploy it', async ({ page, browser }) => {
    await createRegularLocation(browser, firstLocation);
    await loginBasic(page, defaultUserAdmin);
    await openNewRuleForm(page, 'all locations rule');
    await page.getByTestId('field-use_manual_destination_settings').click();
    for (const label of [
      'Include all locations',
      'All users have access',
      'All groups have access',
      'All network devices have access',
      'Enable rule',
    ]) {
      await expect(formToggle(page, label)).toHaveAttribute('data-value', 'true');
    }
    await submitRule(page);

    const row = await openPendingRule(page, 'all locations rule');
    await expect(page.getByTestId('rules-tab-pending')).toHaveText('Pending (1)');
    await expect(
      page.locator('.nav-item', { hasText: 'Rules' }).locator('.counter-label'),
    ).toHaveText('1');
    await expect(page.locator('.header-cell')).toHaveText([
      'Rule name',
      'Predefined destinations',
      'Manually configured destination',
      'Locations',
      'Last modified',
      'Modified by',
      'Status',
    ]);
    await expect(ruleCell(row, 'Locations')).toHaveText('All locations');
    await expect(ruleCell(row, 'Modified by')).toHaveText('admin');
    await expect(ruleCell(row, 'Status')).toHaveText('New');

    await page.getByRole('button', { name: 'Deploy all pending (1)' }).click();
    await expect(row).toBeHidden();
    await expect(page.getByTestId('rules-tab-pending')).toHaveText('Pending');
    await expect(
      page.locator('.nav-item', { hasText: 'Rules' }).locator('.counter-label'),
    ).toHaveCount(0);

    await page.reload();
    await page.getByTestId('rules-tab-deployed').click();
    const deployed = tableRow(page, 'all locations rule');
    await expect(ruleCell(deployed, 'Locations')).toHaveText('All locations');
    await expect(ruleCell(deployed, 'Status')).toHaveText('Active');
  });

  test('Add rule for specific location', async ({ page, browser }) => {
    await createRegularLocation(browser, firstLocation);
    await createRegularLocation(browser, secondLocation);
    await loginBasic(page, defaultUserAdmin);
    await openNewRuleForm(page, 'office rule');
    await page.getByTestId('field-use_manual_destination_settings').click();
    await formToggle(page, 'Include all locations').click();
    await page.getByRole('button', { name: 'Edit locations' }).click();

    const modal = page.locator('#selection-modal');
    await modal.getByPlaceholder('Filter/Search').fill(firstLocation.name);
    await expect(modal.locator('.item', { hasText: secondLocation.name })).toHaveCount(0);
    await selectInModal(page, 'Select locations', [firstLocation.name]);
    await expect(permissionChips(page, 'Include all locations')).toHaveText([
      firstLocation.name,
    ]);
    await submitRule(page);

    const row = await openPendingRule(page, 'office rule');
    await expect(ruleCell(row, 'Locations')).toHaveText(firstLocation.name);
  });

  test('Add rule with manual destination', async ({ page }) => {
    await loginBasic(page, defaultUserAdmin);
    await openNewRuleForm(page, 'manual rule');
    await page.getByTestId('field-use_manual_destination_settings').click();
    for (const label of ['Any address', 'Any port', 'Any protocol']) {
      await expect(formToggle(page, label)).toHaveAttribute('data-value', 'true');
    }
    for (const field of ['field-addresses', 'field-ports', 'field-protocols-6']) {
      await expect(fieldFold(page, field)).toHaveClass('fold folded');
    }

    await formToggle(page, 'Any address').click();
    await expect(fieldFold(page, 'field-addresses')).toHaveClass('fold');
    await addAddresses(page, ['10.5.0.0/24', '10.6.0.1']);
    await formToggle(page, 'Any port').click();
    await expect(fieldFold(page, 'field-ports')).toHaveClass('fold');
    await page.getByTestId('field-ports').fill('80, 443');
    await formToggle(page, 'Any protocol').click();
    await expect(fieldFold(page, 'field-protocols-6')).toHaveClass('fold');
    await page.getByTestId('field-protocols-6').click();
    await page.getByTestId('field-protocols-17').click();
    await submitRule(page);

    const row = await openPendingRule(page, 'manual rule');
    await expect(ruleCell(row, 'Predefined destinations')).toHaveText('');
    await expect(ruleCell(row, 'Manually configured destination')).toHaveText(
      '10.5.0.0/24, 10.6.0.1',
    );

    await openRuleEdit(page, 'manual rule');
    await expect(
      page
        .locator('.multi-select')
        .filter({ has: page.getByTestId('field-addresses') })
        .locator('.chip'),
    ).toHaveText(['10.5.0.0/24', '10.6.0.1']);
    await expect(page.getByTestId('field-ports')).toHaveValue('80, 443');
    await expect(page.getByTestId('field-protocols-6')).toHaveAttribute(
      'data-value',
      'true',
    );
    await expect(page.getByTestId('field-protocols-17')).toHaveAttribute(
      'data-value',
      'true',
    );
    await expect(page.getByTestId('field-protocols-1')).toHaveAttribute(
      'data-value',
      'false',
    );
  });

  test('Add rule with predefined destination', async ({ page }) => {
    await loginBasic(page, defaultUserAdmin);
    await apiCreateDestination(page, 'web servers');
    await apiCreateDestination(page, 'db servers');
    await openNewRuleForm(page, 'destination rule');
    await page.getByRole('button', { name: 'Select predefined destination(s)' }).click();

    const modal = page.locator('#selection-modal');
    await expect(
      modal.locator('.item', { hasText: 'web servers' }).locator('.info'),
    ).toHaveText(['443', 'TCP', '10.20.0.0/24']);
    await modal.getByPlaceholder('Filter/Search').fill('web');
    await expect(modal.locator('.item', { hasText: 'db servers' })).toHaveCount(0);
    await selectInModal(page, 'Select predefined destination(s)', ['web servers']);
    await expect(page.locator('.selected-destinations .name')).toHaveText([
      'web servers',
    ]);
    await submitRule(page);

    const row = await openPendingRule(page, 'destination rule');
    await expect(ruleCell(row, 'Predefined destinations')).toHaveText('web servers');
    await expect(ruleCell(row, 'Manually configured destination')).toHaveText('');
  });

  test('Add rule with predefined and manual destination', async ({ page }) => {
    await loginBasic(page, defaultUserAdmin);
    await apiCreateDestination(page, 'web servers');
    await openNewRuleForm(page, 'mixed rule');
    await expect(page.locator('.divider.text')).toHaveText('or/and');
    await page.getByRole('button', { name: 'Select predefined destination(s)' }).click();
    await selectInModal(page, 'Select predefined destination(s)', ['web servers']);
    await page.getByTestId('field-use_manual_destination_settings').click();
    await formToggle(page, 'Any address').click();
    await addAddresses(page, ['10.5.0.0/24']);
    await submitRule(page);

    const row = await openPendingRule(page, 'mixed rule');
    await expect(ruleCell(row, 'Predefined destinations')).toHaveText('web servers');
    await expect(ruleCell(row, 'Manually configured destination')).toHaveText(
      '10.5.0.0/24',
    );
  });

  test('Add rule with alias', async ({ page }) => {
    await loginBasic(page, defaultUserAdmin);
    await apiCreateAlias(page, 'web ports');
    await apiCreateAlias(page, 'db ports');
    await openNewRuleForm(page, 'alias rule');
    await page.getByTestId('field-use_manual_destination_settings').click();
    await page.getByRole('button', { name: 'Apply aliases' }).click();

    const modal = page.locator('#selection-modal');
    await modal.getByPlaceholder('Filter/Search').fill('web');
    await expect(modal.locator('.item', { hasText: 'db ports' })).toHaveCount(0);
    await selectInModal(page, 'Select aliases', ['web ports']);
    await expect(page.locator('.aliases-selected .chip')).toHaveText(['web ports']);
    await submitRule(page);

    const row = await openPendingRule(page, 'alias rule');
    const manual = ruleCell(row, 'Manually configured destination');
    await expect(manual.getByText('Any address', { exact: true })).toBeVisible();
    await expect(manual.getByText('web ports', { exact: true })).toBeVisible();
  });

  test('Add rule with permissions for specific users', async ({ page, browser }) => {
    await createUser(browser, testUser);
    await loginBasic(page, defaultUserAdmin);
    await openNewRuleForm(page, 'users rule');
    await page.getByTestId('field-use_manual_destination_settings').click();
    await formToggle(page, 'All users have access').click();
    await page.getByRole('button', { name: 'Edit users' }).first().click();
    await selectInModal(page, 'Select allowed users', [testUser.username]);
    await expect(permissionChips(page, 'All users have access')).toHaveText([
      testUser.username,
    ]);
    await submitRule(page);

    await openPendingRule(page, 'users rule');
    await openRuleEdit(page, 'users rule');
    await expect(formToggle(page, 'All users have access')).toHaveAttribute(
      'data-value',
      'false',
    );
    await expect(permissionChips(page, 'All users have access')).toHaveText([
      testUser.username,
    ]);
  });

  test('Add rule with permissions for specific groups', async ({ page, browser }) => {
    await createGroup(browser, 'developers');
    await loginBasic(page, defaultUserAdmin);
    await openNewRuleForm(page, 'groups rule');
    await page.getByTestId('field-use_manual_destination_settings').click();
    await formToggle(page, 'All groups have access').click();
    await page.getByRole('button', { name: 'Edit groups' }).first().click();
    await selectInModal(page, 'Select allowed groups', ['developers']);
    await expect(permissionChips(page, 'All groups have access')).toHaveText([
      'developers',
    ]);
    await submitRule(page);

    await openPendingRule(page, 'groups rule');
    await openRuleEdit(page, 'groups rule');
    await expect(formToggle(page, 'All groups have access')).toHaveAttribute(
      'data-value',
      'false',
    );
    await expect(permissionChips(page, 'All groups have access')).toHaveText([
      'developers',
    ]);
  });

  for (const target of ['users', 'groups']) {
    test(`Add rule excluding all ${target}`, async ({ page }) => {
      await loginBasic(page, defaultUserAdmin);
      await openNewRuleForm(page, `exclude all ${target}`);
      await page.getByTestId('field-use_manual_destination_settings').click();
      await page.getByTestId(`field-restrict_${target}`).click();
      await expect(page.getByTestId(`field-deny_all_${target}-false`)).toBeVisible();
      await page.getByTestId(`field-deny_all_${target}-true`).click();
      await submitRule(page);

      await openPendingRule(page, `exclude all ${target}`);
      await openRuleEdit(page, `exclude all ${target}`);
      await expect(page.getByTestId(`field-restrict_${target}`)).toHaveAttribute(
        'data-active',
        'true',
      );
      await expect(page.getByTestId(`field-deny_all_${target}-true`)).toHaveAttribute(
        'data-active',
        'true',
      );
    });
  }

  test('Add rule excluding specific users', async ({ page, browser }) => {
    await createUser(browser, testUser);
    await loginBasic(page, defaultUserAdmin);
    await openNewRuleForm(page, 'exclude user');
    await page.getByTestId('field-use_manual_destination_settings').click();
    await page.getByTestId('field-restrict_users').click();
    await page.getByTestId('field-deny_all_users-false').click();
    await restriction(page, 'users').getByRole('button', { name: 'Edit users' }).click();
    await selectInModal(page, 'Select restricted users', [testUser.username]);
    await expect(restriction(page, 'users').locator('.selected .chip')).toHaveText([
      testUser.username,
    ]);
    await submitRule(page);

    await openPendingRule(page, 'exclude user');
    await openRuleEdit(page, 'exclude user');
    await expect(page.getByTestId('field-deny_all_users-false')).toHaveAttribute(
      'data-active',
      'true',
    );
    await expect(restriction(page, 'users').locator('.selected .chip')).toHaveText([
      testUser.username,
    ]);
  });

  test('Add rule excluding specific groups', async ({ page, browser }) => {
    await createGroup(browser, 'contractors');
    await loginBasic(page, defaultUserAdmin);
    await openNewRuleForm(page, 'exclude group');
    await page.getByTestId('field-use_manual_destination_settings').click();
    await page.getByTestId('field-restrict_groups').click();
    await page.getByTestId('field-deny_all_groups-false').click();
    await restriction(page, 'groups')
      .getByRole('button', { name: 'Edit groups' })
      .click();
    await selectInModal(page, 'Select restricted groups', ['contractors']);
    await expect(restriction(page, 'groups').locator('.selected .chip')).toHaveText([
      'contractors',
    ]);
    await submitRule(page);

    await openPendingRule(page, 'exclude group');
    await openRuleEdit(page, 'exclude group');
    await expect(page.getByTestId('field-deny_all_groups-false')).toHaveAttribute(
      'data-active',
      'true',
    );
    await expect(restriction(page, 'groups').locator('.selected .chip')).toHaveText([
      'contractors',
    ]);
  });

  test.describe('Network devices', () => {
    test.beforeEach(async ({ browser }) => {
      await createRegularLocation(browser, firstLocation);
      await startNetworkDeviceEnrollment(browser, defaultUserAdmin, { name: 'printer' });
    });

    test('Add rule with permissions for specific network devices', async ({ page }) => {
      await loginBasic(page, defaultUserAdmin);
      await openNewRuleForm(page, 'devices rule');
      await page.getByTestId('field-use_manual_destination_settings').click();
      await formToggle(page, 'All network devices have access').click();
      await page.getByRole('button', { name: 'Edit network devices' }).first().click();
      await selectInModal(page, 'Select allowed network devices', ['printer']);
      await expect(permissionChips(page, 'All network devices have access')).toHaveText([
        'printer',
      ]);
      await submitRule(page);

      await openPendingRule(page, 'devices rule');
      await openRuleEdit(page, 'devices rule');
      await expect(formToggle(page, 'All network devices have access')).toHaveAttribute(
        'data-value',
        'false',
      );
      await expect(permissionChips(page, 'All network devices have access')).toHaveText([
        'printer',
      ]);
    });

    test('Add rule excluding all network devices', async ({ page }) => {
      await loginBasic(page, defaultUserAdmin);
      await openNewRuleForm(page, 'exclude all devices');
      await page.getByTestId('field-use_manual_destination_settings').click();
      await page.getByTestId('field-restrict_devices').click();
      await page.getByTestId('field-deny_all_network_devices-true').click();
      await submitRule(page);

      await openPendingRule(page, 'exclude all devices');
      await openRuleEdit(page, 'exclude all devices');
      await expect(page.getByTestId('field-restrict_devices')).toHaveAttribute(
        'data-active',
        'true',
      );
      await expect(
        page.getByTestId('field-deny_all_network_devices-true'),
      ).toHaveAttribute('data-active', 'true');
    });

    test('Add rule excluding specific network devices', async ({ page }) => {
      await loginBasic(page, defaultUserAdmin);
      await openNewRuleForm(page, 'exclude device');
      await page.getByTestId('field-use_manual_destination_settings').click();
      await page.getByTestId('field-restrict_devices').click();
      await page.getByTestId('field-deny_all_network_devices-false').click();
      await restriction(page, 'devices')
        .getByRole('button', { name: 'Edit network devices' })
        .click();
      await selectInModal(page, 'Select restricted network devices', ['printer']);
      await submitRule(page);

      await openPendingRule(page, 'exclude device');
      await openRuleEdit(page, 'exclude device');
      await expect(
        page.getByTestId('field-deny_all_network_devices-false'),
      ).toHaveAttribute('data-active', 'true');
      await expect(restriction(page, 'devices').locator('.selected .chip')).toHaveText([
        'printer',
      ]);
    });

    test('Rule without any allowed users, groups or devices is rejected', async ({
      page,
    }) => {
      await loginBasic(page, defaultUserAdmin);
      await openNewRuleForm(page, 'no permissions');
      await page.getByTestId('field-use_manual_destination_settings').click();
      await formToggle(page, 'All users have access').click();
      await formToggle(page, 'All groups have access').click();
      await formToggle(page, 'All network devices have access').click();
      await page.getByRole('button', { name: 'Create rule' }).click();
      await expect(
        page.getByText('Configure at least one allowed user, group, or network device.'),
      ).toHaveCount(3);
      await expect(page.getByTestId('field-name')).toHaveValue('no permissions');

      await page.goto(routes.base + routes.firewall.rules);
      await page.getByTestId('rules-tab-pending').click();
      await expect(page.getByText("You don't have any pending rules.")).toBeVisible();
    });

    test('Editing deployed rule keeps all its settings after deploy', async ({
      page,
      browser,
    }) => {
      await createUser(browser, testUser);
      await createGroup(browser, 'developers');
      await createGroup(browser, 'contractors');
      await startNetworkDeviceEnrollment(browser, defaultUserAdmin, { name: 'scanner' });
      await loginBasic(page, defaultUserAdmin);
      await apiCreateDestination(page, 'web servers');
      await apiCreateAlias(page, 'web ports');

      await openNewRuleForm(page, 'full rule');
      await formToggle(page, 'Include all locations').click();
      await page.getByRole('button', { name: 'Edit locations' }).click();
      await selectInModal(page, 'Select locations', [firstLocation.name]);
      await page
        .getByRole('button', { name: 'Select predefined destination(s)' })
        .click();
      await selectInModal(page, 'Select predefined destination(s)', ['web servers']);
      await page.getByTestId('field-use_manual_destination_settings').click();
      await page.getByRole('button', { name: 'Apply aliases' }).click();
      await selectInModal(page, 'Select aliases', ['web ports']);
      await formToggle(page, 'Any address').click();
      await addAddresses(page, ['10.5.0.0/24']);
      await formToggle(page, 'Any port').click();
      await page.getByTestId('field-ports').fill('80, 443');
      await formToggle(page, 'Any protocol').click();
      await page.getByTestId('field-protocols-6').click();

      await formToggle(page, 'All users have access').click();
      await page.getByRole('button', { name: 'Edit users' }).first().click();
      await selectInModal(page, 'Select allowed users', [testUser.username]);
      await formToggle(page, 'All groups have access').click();
      await page.getByRole('button', { name: 'Edit groups' }).first().click();
      await selectInModal(page, 'Select allowed groups', ['developers']);
      await formToggle(page, 'All network devices have access').click();
      await page.getByRole('button', { name: 'Edit network devices' }).first().click();
      await selectInModal(page, 'Select allowed network devices', ['printer']);

      await page.getByTestId('field-restrict_users').click();
      await page.getByTestId('field-deny_all_users-false').click();
      await restriction(page, 'users')
        .getByRole('button', { name: 'Edit users' })
        .click();
      await selectInModal(page, 'Select restricted users', [defaultUserAdmin.username]);
      await page.getByTestId('field-restrict_groups').click();
      await page.getByTestId('field-deny_all_groups-false').click();
      await restriction(page, 'groups')
        .getByRole('button', { name: 'Edit groups' })
        .click();
      await selectInModal(page, 'Select restricted groups', ['contractors']);
      await page.getByTestId('field-restrict_devices').click();
      await page.getByTestId('field-deny_all_network_devices-false').click();
      await restriction(page, 'devices')
        .getByRole('button', { name: 'Edit network devices' })
        .click();
      await selectInModal(page, 'Select restricted network devices', ['scanner']);
      await submitRule(page);

      await openPendingRule(page, 'full rule');
      await page.getByRole('button', { name: 'Deploy all pending (1)' }).click();
      await expect(tableRow(page, 'full rule')).toBeHidden();

      await page.goto(routes.base + routes.firewall.rules);
      await openRuleEdit(page, 'full rule');
      await page.getByTestId('field-name').fill('full rule edited');
      await page.getByRole('button', { name: 'Save changes' }).click();
      await page.waitForURL('**' + routes.firewall.rules + '**');

      const pending = await openPendingRule(page, 'full rule edited');
      await expect(ruleCell(pending, 'Status')).toHaveText('Modified');
      await page.getByRole('button', { name: 'Deploy all pending (1)' }).click();
      await expect(pending).toBeHidden();

      await page.goto(routes.base + routes.firewall.rules);
      const deployed = tableRow(page, 'full rule edited');
      await expect(ruleCell(deployed, 'Status')).toHaveText('Active');
      await expect(ruleCell(deployed, 'Locations')).toHaveText(firstLocation.name);
      await expect(ruleCell(deployed, 'Predefined destinations')).toHaveText(
        'web servers',
      );
      await openRuleEdit(page, 'full rule edited');

      await expect(permissionChips(page, 'Include all locations')).toHaveText([
        firstLocation.name,
      ]);
      await expect(page.locator('.selected-destinations .name')).toHaveText([
        'web servers',
      ]);
      await expect(page.locator('.aliases-selected .chip')).toHaveText(['web ports']);
      await expect(
        page
          .locator('.multi-select')
          .filter({ has: page.getByTestId('field-addresses') })
          .locator('.chip'),
      ).toHaveText(['10.5.0.0/24']);
      await expect(page.getByTestId('field-ports')).toHaveValue('80, 443');
      await expect(page.getByTestId('field-protocols-6')).toHaveAttribute(
        'data-value',
        'true',
      );
      await expect(page.getByTestId('field-protocols-17')).toHaveAttribute(
        'data-value',
        'false',
      );
      await expect(permissionChips(page, 'All users have access')).toHaveText([
        testUser.username,
      ]);
      await expect(permissionChips(page, 'All groups have access')).toHaveText([
        'developers',
      ]);
      await expect(permissionChips(page, 'All network devices have access')).toHaveText([
        'printer',
      ]);
      await expect(restriction(page, 'users').locator('.selected .chip')).toHaveText([
        defaultUserAdmin.username,
      ]);
      await expect(restriction(page, 'groups').locator('.selected .chip')).toHaveText([
        'contractors',
      ]);
      await expect(restriction(page, 'devices').locator('.selected .chip')).toHaveText([
        'scanner',
      ]);
    });
  });

  test('Add disabled rule', async ({ page }) => {
    await loginBasic(page, defaultUserAdmin);
    await openNewRuleForm(page, 'disabled rule');
    await page.getByTestId('field-use_manual_destination_settings').click();
    await formToggle(page, 'Enable rule').click();
    await submitRule(page);

    const row = await openPendingRule(page, 'disabled rule');
    await expect(ruleCell(row, 'Status')).toHaveText('New');
    await page.getByRole('button', { name: 'Deploy all pending (1)' }).click();
    await expect(row).toBeHidden();

    await page.reload();
    await page.getByTestId('rules-tab-deployed').click();
    await expect(ruleCell(tableRow(page, 'disabled rule'), 'Status')).toHaveText(
      'Disabled',
    );
  });

  test('Rule without name and destination is rejected', async ({ page }) => {
    await loginBasic(page, defaultUserAdmin);
    await openNewRuleForm(page, '');
    await page.getByRole('button', { name: 'Create rule' }).click();
    await expect(page.getByText('Field is required')).toBeVisible();
    await expect(page.getByText('Configure manual destination.')).toBeVisible();

    await page.getByTestId('field-name').fill('empty manual');
    await page.getByTestId('field-use_manual_destination_settings').click();
    await formToggle(page, 'Any address').click();
    await page.getByRole('button', { name: 'Create rule' }).click();
    await expect(
      page.getByText(
        'Manual destination settings are enabled. Provide a value or enable Any.',
      ),
    ).toBeVisible();

    await page.goto(routes.base + routes.firewall.rules);
    await page.getByTestId('rules-tab-pending').click();
    await expect(page.getByText("You don't have any pending rules.")).toBeVisible();
  });

  test('Rule allowing and excluding the same user is rejected', async ({
    page,
    browser,
  }) => {
    await createUser(browser, testUser);
    await loginBasic(page, defaultUserAdmin);
    await openNewRuleForm(page, 'conflict rule');
    await page.getByTestId('field-use_manual_destination_settings').click();
    await formToggle(page, 'All users have access').click();
    await page.getByRole('button', { name: 'Edit users' }).first().click();
    await selectInModal(page, 'Select allowed users', [testUser.username]);
    await page.getByTestId('field-restrict_users').click();
    await page.getByTestId('field-deny_all_users-false').click();
    await restriction(page, 'users').getByRole('button', { name: 'Edit users' }).click();
    await selectInModal(page, 'Select restricted users', [testUser.username]);
    await page.getByRole('button', { name: 'Create rule' }).click();
    await expect(
      page.getByText('The same entry cannot be both allowed and restricted.').first(),
    ).toBeVisible();

    await page.goto(routes.base + routes.firewall.rules);
    await page.getByTestId('rules-tab-pending').click();
    await expect(page.getByText("You don't have any pending rules.")).toBeVisible();
  });

  test('Search rules', async ({ page }) => {
    await loginBasic(page, defaultUserAdmin);
    const ids = [
      await apiCreateRule(page, 'web rule'),
      await apiCreateRule(page, 'db rule'),
    ];
    await apiApplyRules(page, ids);

    await page.goto(routes.base + routes.firewall.rules);
    await expect(tableRow(page, 'db rule')).toBeVisible();
    await page.getByPlaceholder('Search').fill('web');
    await expect(tableRow(page, 'web rule')).toBeVisible();
    await expect(tableRow(page, 'db rule')).toHaveCount(0);
  });

  test('Edit pending rule', async ({ page }) => {
    await loginBasic(page, defaultUserAdmin);
    await apiCreateRule(page, 'pending rule');

    await openPendingRule(page, 'pending rule');
    await openRuleEdit(page, 'pending rule');
    await page.getByTestId('field-name').fill('renamed rule');
    await page.getByRole('button', { name: 'Save changes' }).click();
    await page.waitForURL('**' + routes.firewall.rules + '**');

    const row = await openPendingRule(page, 'renamed rule');
    await expect(ruleCell(row, 'Status')).toHaveText('New');
    await expect(page.getByTestId('rules-tab-pending')).toHaveText('Pending (1)');
    await expect(tableRow(page, 'pending rule')).toHaveCount(0);
    await page.getByTestId('rules-tab-deployed').click();
    await expect(page.getByText("You don't have any firewall rules yet.")).toBeVisible();
  });

  test('Delete pending rule', async ({ page }) => {
    await loginBasic(page, defaultUserAdmin);
    await apiCreateRule(page, 'pending rule');

    const row = await openPendingRule(page, 'pending rule');
    await row.locator('.icon-button').click();
    await page.locator('.menu').getByText('Delete', { exact: true }).click();
    await expect(row).toBeHidden();

    await page.reload();
    await expect(page.getByText("You don't have any pending rules.")).toBeVisible();
    await page.getByTestId('rules-tab-deployed').click();
    await expect(page.getByText("You don't have any firewall rules yet.")).toBeVisible();
  });

  test('Edit deployed rule', async ({ page }) => {
    await loginBasic(page, defaultUserAdmin);
    const id = await apiCreateRule(page, 'old name');
    await apiApplyRules(page, [id]);

    await page.goto(routes.base + routes.firewall.rules);
    await openRuleEdit(page, 'old name');
    await page.getByTestId('field-name').fill('new name');
    await page.getByRole('button', { name: 'Save changes' }).click();
    await page.waitForURL('**' + routes.firewall.rules + '**');

    const pending = await openPendingRule(page, 'new name');
    await expect(ruleCell(pending, 'Status')).toHaveText('Modified');
    await page.getByTestId('rules-tab-deployed').click();
    await expect(tableRow(page, 'old name')).toBeVisible();
    await expect(tableRow(page, 'new name')).toHaveCount(0);

    await page.getByTestId('rules-tab-pending').click();
    await page.getByRole('button', { name: 'Deploy all pending (1)' }).click();
    await expect(tableRow(page, 'new name')).toBeHidden();
    await page.reload();
    await page.getByTestId('rules-tab-deployed').click();
    await expect(tableRow(page, 'new name')).toBeVisible();
    await expect(tableRow(page, 'old name')).toHaveCount(0);
  });

  test('Deleted deployed rule is pending as Deleted', async ({ page }) => {
    await loginBasic(page, defaultUserAdmin);
    const id = await apiCreateRule(page, 'to delete');
    await apiApplyRules(page, [id]);

    await page.goto(routes.base + routes.firewall.rules);
    await tableRow(page, 'to delete').locator('.icon-button').click();
    await page.locator('.menu').getByText('Delete', { exact: true }).click();
    await expect(page.getByTestId('rules-tab-pending')).toHaveText('Pending (1)');

    const row = await openPendingRule(page, 'to delete');
    await expect(ruleCell(row, 'Status')).toHaveText('Deleted');
  });
});
