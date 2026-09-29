import { expect, test } from '@playwright/test';

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

test.describe('ACL duplicate', () => {
  test.beforeEach(() => {
    test.skip(!process.env.DEFGUARD_LICENSE_KEY, 'ACL is a business feature');
    dockerRestart();
  });

  test('Duplicate deployed alias', async ({ page }) => {
    const ALIAS_NAME = 'alias-one';
    const ALIAS_COPY_NAME = `Copy of ${ALIAS_NAME}`;

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateAlias(page, ALIAS_NAME);

    await page.goto(routes.base + routes.firewall.aliases);
    await page
      .locator('.virtual-row')
      .filter({ hasText: ALIAS_NAME })
      .locator('.icon-button')
      .click();
    await page.getByTestId('alias-row-duplicate').click();

    await page.waitForURL(routes.base + routes.firewall.addAlias + '**');
    await expect(page.getByTestId('field-name')).toHaveValue(ALIAS_COPY_NAME);
    await expect(page.getByTestId('field-addresses')).toHaveValue('10.10.0.0/24');
    await expect(page.getByTestId('field-ports')).toHaveValue('443');

    await page.locator('button[type="submit"]').click();
    await page.waitForURL(routes.base + routes.firewall.aliases + '**');

    const aliases = await apiGetAliases(page);
    expect(aliases.map((alias) => alias.name).sort()).toEqual([
      ALIAS_COPY_NAME,
      ALIAS_NAME,
    ]);
    const copy = aliases.find((alias) => alias.name === ALIAS_COPY_NAME);
    expect(copy).toMatchObject({
      state: 'Applied',
      addresses: '10.10.0.0/24',
      ports: '443',
      protocols: [6],
    });
  });

  test('Cancelling alias duplicate does not create an alias', async ({ page }) => {
    const ALIAS_NAME = 'alias-one';
    const ALIAS_COPY_NAME = `Copy of ${ALIAS_NAME}`;

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateAlias(page, ALIAS_NAME);

    await page.goto(routes.base + routes.firewall.aliases);
    await page
      .locator('.virtual-row')
      .filter({ hasText: ALIAS_NAME })
      .locator('.icon-button')
      .click();
    await page.getByTestId('alias-row-duplicate').click();
    await page.waitForURL(routes.base + routes.firewall.addAlias + '**');
    await expect(page.getByTestId('field-name')).toHaveValue(ALIAS_COPY_NAME);

    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.waitForURL(routes.base + routes.firewall.aliases + '**');

    await page.getByTestId('aliases-tab-deployed').click();
    await expect(page.getByText(ALIAS_COPY_NAME)).not.toBeVisible();
    const items = await apiGetAliases(page);
    expect(items.map((item) => item.name)).toEqual([ALIAS_NAME]);
  });

  test('Duplicate is not available for pending aliases', async ({ page }) => {
    const ALIAS_NAME = 'alias-one';
    const ALIAS_EDITED_NAME = 'alias-one-edited';

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const id = await apiCreateAlias(page, ALIAS_NAME);
    await apiModifyAlias(page, id, ALIAS_EDITED_NAME);

    await page.goto(routes.base + routes.firewall.aliases);
    await page.getByTestId('aliases-tab-pending').click();
    await page
      .locator('.virtual-row')
      .filter({ hasText: ALIAS_EDITED_NAME })
      .locator('.icon-button')
      .click();
    await expect(page.getByTestId('alias-row-deploy')).toBeVisible();
    await expect(page.getByTestId('alias-row-duplicate')).toHaveCount(0);
  });

  test('Duplicate deployed destination', async ({ page }) => {
    const DESTINATION_NAME = 'destination-one';
    const DESTINATION_COPY_NAME = `Copy of ${DESTINATION_NAME}`;

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateDestination(page, DESTINATION_NAME);

    await page.goto(routes.base + routes.firewall.destinations);
    await page
      .locator('.virtual-row')
      .filter({ hasText: DESTINATION_NAME })
      .locator('.icon-button')
      .click();
    await page.getByTestId('destination-row-duplicate').click();

    await page.waitForURL(routes.base + routes.firewall.addDestination + '**');
    await expect(page.getByTestId('field-name')).toHaveValue(DESTINATION_COPY_NAME);
    await expect(page.getByLabel('IPv4/IPv6 CIDR ranges or addresses')).toHaveValue(
      '10.20.0.0/24',
    );
    await expect(page.getByTestId('field-ports')).toHaveValue('443');

    await page.locator('button[type="submit"]').click();
    await page.waitForURL(routes.base + routes.firewall.destinations + '**');

    const destinations = await apiGetDestinations(page);
    expect(destinations.map((destination) => destination.name).sort()).toEqual([
      DESTINATION_COPY_NAME,
      DESTINATION_NAME,
    ]);
    expect(destinations.every((destination) => destination.state === 'Applied')).toBe(
      true,
    );
    const copy = destinations.find(
      (destination) => destination.name === DESTINATION_COPY_NAME,
    );
    expect(copy).toMatchObject({
      addresses: '10.20.0.0/24',
      ports: '443',
      protocols: [6],
      any_address: false,
      any_port: false,
      any_protocol: false,
    });
  });

  test('Cancelling destination duplicate does not create a destination', async ({
    page,
  }) => {
    const DESTINATION_NAME = 'destination-one';
    const DESTINATION_COPY_NAME = `Copy of ${DESTINATION_NAME}`;

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateDestination(page, DESTINATION_NAME);

    await page.goto(routes.base + routes.firewall.destinations);
    await page
      .locator('.virtual-row')
      .filter({ hasText: DESTINATION_NAME })
      .locator('.icon-button')
      .click();
    await page.getByTestId('destination-row-duplicate').click();
    await page.waitForURL(routes.base + routes.firewall.addDestination + '**');
    await expect(page.getByTestId('field-name')).toHaveValue(DESTINATION_COPY_NAME);

    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.waitForURL(routes.base + routes.firewall.destinations + '**');

    await page.getByTestId('destinations-tab-deployed').click();
    await expect(page.getByText(DESTINATION_COPY_NAME)).not.toBeVisible();
    const items = await apiGetDestinations(page);
    expect(items.map((item) => item.name)).toEqual([DESTINATION_NAME]);
  });

  test('Duplicate is not available for pending destinations', async ({ page }) => {
    const DESTINATION_NAME = 'destination-one';
    const DESTINATION_EDITED_NAME = 'destination-one-edited';

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const id = await apiCreateDestination(page, DESTINATION_NAME);
    await apiModifyDestination(page, id, DESTINATION_EDITED_NAME);

    await page.goto(routes.base + routes.firewall.destinations);
    await page.getByTestId('destinations-tab-pending').click();
    await page
      .locator('.virtual-row')
      .filter({ hasText: DESTINATION_EDITED_NAME })
      .locator('.icon-button')
      .click();
    await expect(page.getByTestId('destination-row-deploy')).toBeVisible();
    await expect(page.getByTestId('destination-row-duplicate')).toHaveCount(0);
  });

  test('Duplicate deployed rule', async ({ page }) => {
    const RULE_NAME = 'rule-one';
    const RULE_COPY_NAME = `Copy of ${RULE_NAME}`;

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const id = await apiCreateRule(page, RULE_NAME, {
      addresses: '10.30.0.0/24',
      ports: '80',
      protocols: [6],
      any_address: false,
      any_port: false,
      any_protocol: false,
    });
    await apiApplyRules(page, [id]);

    await page.goto(routes.base + routes.firewall.rules);
    await page
      .locator('.virtual-row')
      .filter({ hasText: RULE_NAME })
      .locator('.icon-button')
      .click();
    await page.getByTestId('rule-row-duplicate').click();

    await page.waitForURL(routes.base + routes.firewall.addRule + '**');
    await expect(page.getByTestId('field-name')).toHaveValue(RULE_COPY_NAME);
    await expect(page.getByLabel('IPv4/IPv6 CIDR ranges or addresses')).toHaveValue(
      '10.30.0.0/24',
    );
    await expect(page.getByTestId('field-ports')).toHaveValue('80');

    await page.locator('button[type="submit"]').click();
    await page.waitForURL(routes.base + routes.firewall.rules + '**');

    const rules = await apiGetRules(page);
    expect(rules.map((rule) => rule.name).sort()).toEqual([RULE_COPY_NAME, RULE_NAME]);
    const copy = rules.find((rule) => rule.name === RULE_COPY_NAME);
    expect(copy).toMatchObject({
      state: 'New',
      enabled: true,
      addresses: '10.30.0.0/24',
      ports: '80',
      protocols: [6],
      any_address: false,
      any_port: false,
      any_protocol: false,
    });
  });

  test('Cancelling rule duplicate does not create a rule', async ({ page }) => {
    const RULE_NAME = 'rule-one';
    const RULE_COPY_NAME = `Copy of ${RULE_NAME}`;

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    const id = await apiCreateRule(page, RULE_NAME);
    await apiApplyRules(page, [id]);

    await page.goto(routes.base + routes.firewall.rules);
    await page
      .locator('.virtual-row')
      .filter({ hasText: RULE_NAME })
      .locator('.icon-button')
      .click();
    await page.getByTestId('rule-row-duplicate').click();
    await page.waitForURL(routes.base + routes.firewall.addRule + '**');
    await expect(page.getByTestId('field-name')).toHaveValue(RULE_COPY_NAME);

    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.waitForURL(routes.base + routes.firewall.rules + '**');

    await page.getByTestId('rules-tab-pending').click();
    await expect(page.getByText(RULE_COPY_NAME)).not.toBeVisible();
    const rules = await apiGetRules(page);
    expect(rules.map((rule) => rule.name)).toEqual([RULE_NAME]);
  });

  test('Duplicate is not available for pending rules', async ({ page }) => {
    const RULE_NAME = 'rule-one';

    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateRule(page, RULE_NAME);

    await page.goto(routes.base + routes.firewall.rules);
    await page.getByTestId('rules-tab-pending').click();
    await page
      .locator('.virtual-row')
      .filter({ hasText: RULE_NAME })
      .locator('.icon-button')
      .click();
    await expect(page.locator('.menu')).toBeVisible();
    await expect(page.getByTestId('rule-row-duplicate')).toHaveCount(0);
  });
});
