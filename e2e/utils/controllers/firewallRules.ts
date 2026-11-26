import { expect, Locator, Page } from '@playwright/test';

import { routes } from '../../config';

const ruleColumns = [
  'Select',
  'Rule name',
  'Predefined destinations',
  'Manually configured destination',
  'Locations',
  'Last modified',
  'Modified by',
  'Status',
];

export const aliasColumns = [
  'Select',
  'Alias name',
  'IPv4/IPv6 CIDR ranges',
  'Ports',
  'Protocols',
  'Used in rules',
];

export const destinationColumns = [
  'Select',
  'Destination name',
  'IPv4/IPv6 CIDR ranges or addresses',
  'Ports',
  'Protocols',
  'Used in rules',
];

export const tableRow = (page: Page, name: string): Locator =>
  page.locator('.virtual-row').filter({ hasText: name });

export const tableCell = (row: Locator, columns: string[], column: string): Locator =>
  row.locator('.table-cell').nth(columns.indexOf(column));

export const ruleCell = (row: Locator, column: string): Locator =>
  tableCell(row, ruleColumns, column);

export const formToggle = (page: Page, label: string): Locator =>
  page.locator('.toggle', { hasText: label });

export const permissionChips = (page: Page, toggleLabel: string): Locator =>
  page
    .locator('.select-multiple', { has: formToggle(page, toggleLabel) })
    .locator('.selected .chip');

export const restriction = (page: Page, target: string): Locator =>
  page.locator(`.checkbox:has([data-testid="field-restrict_${target}"]) + .fold`);

export const fieldFold = (page: Page, testId: string): Locator =>
  page
    .locator('.fold')
    .filter({ has: page.getByTestId(testId) })
    .last();

export const selectInModal = async (page: Page, title: string, names: string[]) => {
  const modal = page.locator('#selection-modal');
  await expect(modal.locator('.title')).toHaveText(title);
  await expect(modal.getByText('Select all', { exact: true })).toBeVisible();
  await expect(modal.locator('.toggle', { hasText: 'Show selected only' })).toBeVisible();
  for (const name of names) {
    await modal.getByPlaceholder('Filter/Search').fill(name);
    await modal.locator('.item').getByText(name, { exact: true }).click();
  }
  await modal.getByRole('button', { name: 'Submit' }).click();
  await expect(modal).toBeHidden();
};

export const openNewRuleForm = async (page: Page, name: string) => {
  await page.goto(routes.base + routes.firewall.rules);
  await page.getByRole('button', { name: 'Create new rule' }).click();
  await page.waitForURL('**' + routes.firewall.addRule + '**');
  await page.getByTestId('field-name').fill(name);
};

export const addAddresses = async (page: Page, addresses: string[]) => {
  for (const address of addresses) {
    await page.getByTestId('field-addresses').fill(address);
    await page.getByTestId('field-addresses').press('Enter');
  }
};

export const submitRule = async (page: Page) => {
  await page.getByRole('button', { name: 'Create rule' }).click();
  await expect(
    page.getByText('Rule added to Pending tab and awaiting deployment.'),
  ).toBeVisible();
  await page.waitForURL('**' + routes.firewall.rules + '**');
};

export const openPendingRule = async (page: Page, name: string): Promise<Locator> => {
  await page.goto(routes.base + routes.firewall.rules);
  await page.getByTestId('rules-tab-pending').click();
  const row = tableRow(page, name);
  await expect(row).toBeVisible();
  return row;
};

export const openRuleEdit = async (page: Page, name: string) => {
  await tableRow(page, name).locator('.icon-button').click();
  await page.locator('.menu').getByText('Edit', { exact: true }).click();
  await page.waitForURL('**' + routes.firewall.editRule + '**');
  await expect(page.getByTestId('field-name')).toHaveValue(name);
};
