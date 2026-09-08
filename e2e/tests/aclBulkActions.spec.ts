import { expect, Locator, Page, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import {
  apiApplyRules,
  apiCreateAlias,
  apiCreateDestination,
  apiCreateRule,
  apiGetAliases,
  apiGetDestinations,
  apiGetRules,
  apiModifyAlias,
  apiModifyDestination,
} from '../utils/api/acl';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const row = (page: Page, name: string): Locator =>
  page.locator('.virtual-row').filter({ hasText: name });

const select = async (page: Page, ...names: string[]): Promise<void> => {
  for (const name of names) {
    await row(page, name).locator('.checkbox').click();
  }
};

const confirm = async (page: Page): Promise<void> => {
  const modal = page.locator('.modal');
  await modal.waitFor({ state: 'visible' });
  await page.getByTestId('confirm-action-submit').click();
  await modal.waitFor({ state: 'hidden' });
};

const bulkAction = async (page: Page, area: string, action: string): Promise<void> => {
  await page.getByTestId(`${area}-bulk-actions`).click();
  await page.getByTestId(`${area}-bulk-${action}`).click();
};

test.describe('ACL bulk actions', () => {
  test.beforeEach(() => {
    test.skip(
      !process.env.DEFGUARD_LICENSE_KEY,
      'ACL is a business feature; without DEFGUARD_LICENSE_KEY, every action opens the ' +
        'upgrade modal',
    );
    dockerRestart();
  });

  test('Bulk deploy and delete pending rules', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateRule(page, 'rule-one');
    await apiCreateRule(page, 'rule-two');
    await apiCreateRule(page, 'rule-three');

    await page.goto(routes.base + routes.firewall.rules);
    await page.getByTestId('rules-tab-pending').click();
    await expect(row(page, 'rule-one')).toBeVisible();

    await select(page, 'rule-one', 'rule-two');
    await bulkAction(page, 'rules', 'deploy');
    await confirm(page);

    await expect(row(page, 'rule-one')).toHaveCount(0);
    await expect(row(page, 'rule-two')).toHaveCount(0);
    await expect(row(page, 'rule-three')).toBeVisible();

    await page.getByTestId('rules-tab-deployed').click();
    await expect(row(page, 'rule-one')).toContainText('Active');
    await expect(row(page, 'rule-two')).toContainText('Active');

    await page.getByTestId('rules-tab-pending').click();
    await select(page, 'rule-three');
    await bulkAction(page, 'rules', 'delete');
    await confirm(page);

    await expect(row(page, 'rule-three')).toHaveCount(0);
    const remaining = await apiGetRules(page);
    expect(remaining.map((rule) => rule.name).sort()).toEqual(['rule-one', 'rule-two']);
  });

  test('Bulk disable and enable deployed rules', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const ids = [
      await apiCreateRule(page, 'rule-one'),
      await apiCreateRule(page, 'rule-two'),
      await apiCreateRule(page, 'rule-three'),
    ];
    await apiApplyRules(page, ids);

    await page.goto(routes.base + routes.firewall.rules);
    await expect(row(page, 'rule-one')).toContainText('Active');

    // Disabling an applied rule creates a pending `Modified` child, so the parent remains
    // `Active`.
    await select(page, 'rule-one', 'rule-two');
    await bulkAction(page, 'rules', 'disable');
    await confirm(page);
    await expect(row(page, 'rule-one')).toContainText('Active');

    await page.getByTestId('rules-tab-pending').click();
    await expect(row(page, 'rule-one')).toContainText('Modified');
    await expect(row(page, 'rule-two')).toContainText('Modified');
    await expect(row(page, 'rule-three')).toHaveCount(0);

    // Deploying the pending children applies the disable.
    await select(page, 'rule-one', 'rule-two');
    await bulkAction(page, 'rules', 'deploy');
    await confirm(page);

    await page.getByTestId('rules-tab-deployed').click();
    await expect(row(page, 'rule-one')).toContainText('Disabled');
    await expect(row(page, 'rule-two')).toContainText('Disabled');
    await expect(row(page, 'rule-three')).toContainText('Active');

    await select(page, 'rule-one', 'rule-two');
    await bulkAction(page, 'rules', 'enable');
    await confirm(page);

    await page.getByTestId('rules-tab-pending').click();
    await select(page, 'rule-one', 'rule-two');
    await bulkAction(page, 'rules', 'deploy');
    await confirm(page);

    await page.getByTestId('rules-tab-deployed').click();
    await expect(row(page, 'rule-one')).toContainText('Active');
    await expect(row(page, 'rule-two')).toContainText('Active');
  });

  test('Bulk disable warns when every selected rule is already disabled', async ({
    page,
  }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const id = await apiCreateRule(page, 'rule-one', { enabled: false });
    await apiApplyRules(page, [id]);

    await page.goto(routes.base + routes.firewall.rules);
    await expect(row(page, 'rule-one')).toContainText('Disabled');

    await select(page, 'rule-one');
    await bulkAction(page, 'rules', 'disable');

    // The warning replaces the confirmation modal, so no request is sent.
    await expect(page.locator('.snackbar')).toBeVisible();
    await expect(page.locator('.modal')).toBeHidden();
    const rules = await apiGetRules(page);
    expect(rules).toHaveLength(1);
  });

  test('Bulk delete deployed aliases', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateAlias(page, 'alias-one');
    await apiCreateAlias(page, 'alias-two');
    await apiCreateAlias(page, 'alias-three');

    await page.goto(routes.base + routes.firewall.aliases);
    await expect(row(page, 'alias-one')).toBeVisible();

    await select(page, 'alias-one', 'alias-two');
    await bulkAction(page, 'aliases', 'delete');
    await confirm(page);

    await expect(row(page, 'alias-one')).toHaveCount(0);
    await expect(row(page, 'alias-two')).toHaveCount(0);
    await expect(row(page, 'alias-three')).toBeVisible();

    const remaining = await apiGetAliases(page);
    expect(remaining.map((alias) => alias.name)).toEqual(['alias-three']);
  });

  test('Bulk delete skips an alias used by a rule', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const usedId = await apiCreateAlias(page, 'alias-used');
    await apiCreateAlias(page, 'alias-free');
    await apiCreateRule(page, 'rule-one', { aliases: [usedId] });

    await page.goto(routes.base + routes.firewall.aliases);
    await expect(row(page, 'alias-used')).toBeVisible();

    // The used alias is removed from the request, so only the free alias is deleted.
    await select(page, 'alias-used', 'alias-free');
    await bulkAction(page, 'aliases', 'delete');
    await confirm(page);

    await expect(row(page, 'alias-free')).toHaveCount(0);
    await expect(row(page, 'alias-used')).toBeVisible();

    // With no eligible items, the action shows only a warning.
    await select(page, 'alias-used');
    await bulkAction(page, 'aliases', 'delete');
    await expect(page.locator('.snackbar')).toBeVisible();
    await expect(page.locator('.modal')).toBeHidden();

    const remaining = await apiGetAliases(page);
    expect(remaining.map((alias) => alias.name)).toEqual(['alias-used']);
  });

  test('Bulk deploy pending aliases', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const firstId = await apiCreateAlias(page, 'alias-one');
    const secondId = await apiCreateAlias(page, 'alias-two');
    await apiModifyAlias(page, firstId, 'alias-one-edited');
    await apiModifyAlias(page, secondId, 'alias-two-edited');

    await page.goto(routes.base + routes.firewall.aliases);
    await page.getByTestId('aliases-tab-pending').click();
    await expect(row(page, 'alias-one-edited')).toBeVisible();

    await select(page, 'alias-one-edited', 'alias-two-edited');
    await bulkAction(page, 'aliases', 'deploy');
    await confirm(page);

    await expect(row(page, 'alias-one-edited')).toHaveCount(0);
    await page.getByTestId('aliases-tab-deployed').click();
    await expect(row(page, 'alias-one-edited')).toBeVisible();
    await expect(row(page, 'alias-two-edited')).toBeVisible();

    const remaining = await apiGetAliases(page);
    expect(remaining.every((alias) => alias.state === 'Applied')).toBe(true);
  });

  test('Bulk delete deployed destinations', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateDestination(page, 'destination-one');
    await apiCreateDestination(page, 'destination-two');
    await apiCreateDestination(page, 'destination-three');

    await page.goto(routes.base + routes.firewall.destinations);
    await expect(row(page, 'destination-one')).toBeVisible();

    await select(page, 'destination-one', 'destination-two');
    await bulkAction(page, 'destinations', 'delete');
    await confirm(page);

    await expect(row(page, 'destination-one')).toHaveCount(0);
    await expect(row(page, 'destination-two')).toHaveCount(0);
    await expect(row(page, 'destination-three')).toBeVisible();

    const remaining = await apiGetDestinations(page);
    expect(remaining.map((destination) => destination.name)).toEqual([
      'destination-three',
    ]);
  });

  test('Bulk delete skips a destination used by a rule', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const usedId = await apiCreateDestination(page, 'destination-used');
    await apiCreateDestination(page, 'destination-free');
    await apiCreateRule(page, 'rule-one', {
      destinations: [usedId],
      use_manual_destination_settings: false,
    });

    await page.goto(routes.base + routes.firewall.destinations);
    await expect(row(page, 'destination-used')).toBeVisible();

    await select(page, 'destination-used', 'destination-free');
    await bulkAction(page, 'destinations', 'delete');
    await confirm(page);

    await expect(row(page, 'destination-free')).toHaveCount(0);
    await expect(row(page, 'destination-used')).toBeVisible();

    await select(page, 'destination-used');
    await bulkAction(page, 'destinations', 'delete');
    await expect(page.locator('.snackbar')).toBeVisible();
    await expect(page.locator('.modal')).toBeHidden();

    const remaining = await apiGetDestinations(page);
    expect(remaining.map((destination) => destination.name)).toEqual([
      'destination-used',
    ]);
  });

  test('Bulk deploy pending destinations', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const firstId = await apiCreateDestination(page, 'destination-one');
    const secondId = await apiCreateDestination(page, 'destination-two');
    await apiModifyDestination(page, firstId, 'destination-one-edited');
    await apiModifyDestination(page, secondId, 'destination-two-edited');

    await page.goto(routes.base + routes.firewall.destinations);
    await page.getByTestId('destinations-tab-pending').click();
    await expect(row(page, 'destination-one-edited')).toBeVisible();

    await select(page, 'destination-one-edited', 'destination-two-edited');
    await bulkAction(page, 'destinations', 'deploy');
    await confirm(page);

    await expect(row(page, 'destination-one-edited')).toHaveCount(0);
    await page.getByTestId('destinations-tab-deployed').click();
    await expect(row(page, 'destination-one-edited')).toBeVisible();
    await expect(row(page, 'destination-two-edited')).toBeVisible();

    const remaining = await apiGetDestinations(page);
    expect(remaining.every((destination) => destination.state === 'Applied')).toBe(true);
  });
});
