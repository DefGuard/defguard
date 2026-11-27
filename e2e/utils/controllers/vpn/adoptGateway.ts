import { Browser, expect } from '@playwright/test';

import { defaultUserAdmin, routes, testsConfig } from '../../../config';
import { waitForBase } from '../../waitForBase';
import { waitForRoute } from '../../waitForRoute';
import { loginBasic } from '../login';

export const adoptGateway = async (
  browser: Browser,
  locationName: string,
  gatewayName: string,
) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.locations);
  await page
    .locator('.virtual-row')
    .filter({ hasText: locationName })
    .locator('.icon-button')
    .click();
  await page.locator('.menu').getByText('Gateway setup', { exact: true }).click();

  await page.getByRole('button', { name: 'Configure gateway' }).click();
  await page.locator('.checkbox').click();
  const continueButton = page.getByRole('button', { name: 'Continue' });
  await continueButton.click();

  await page.getByTestId('field-common_name').fill(gatewayName);
  await page.getByTestId('field-ip_or_domain').fill('gateway');
  await page.getByRole('button', { name: 'Adopt Gateway component' }).click();
  await expect(page.locator('.loading-step.variant-success')).toHaveCount(6, {
    timeout: testsConfig.TEST_TIMEOUT * 1000,
  });
  await continueButton.click();
  await expect(
    page.getByText('Gateway component configured successfully.'),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Go to locations' }).click();
  await waitForRoute(page, routes.locations);
  const gatewaysTab = page.locator('.tab', { hasText: 'Gateways' });
  const row = page.locator('.virtual-row').filter({ hasText: gatewayName });
  await expect(async () => {
    await page.reload();
    await gatewaysTab.click();
    await expect(row).toContainText('Connected', { timeout: 1_000 });
  }).toPass({ timeout: testsConfig.TEST_TIMEOUT * 1000 });
  await context.close();
};
