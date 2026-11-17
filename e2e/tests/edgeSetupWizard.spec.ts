import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const EXISTING_EDGE_NAME = 'edge-test';

test.describe('Edge setup wizard', () => {
  test.beforeEach(() => {
    test.skip(
      !process.env.DEFGUARD_LICENSE_KEY,
      'Adding a second Edge component requires an Enterprise license ' +
        '(DEFGUARD_LICENSE_KEY)',
    );
    dockerRestart();
  });

  test('Wizard requires deploy confirmation and rejects a duplicate name', async ({
    page,
  }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.edges);
    await expect(
      page.locator('.virtual-row').filter({ hasText: EXISTING_EDGE_NAME }),
    ).toBeVisible();

    await page.getByTestId('add-edge').click();
    await page.getByRole('button', { name: 'Configure Edge component' }).click();

    const continueButton = page.getByRole('button', { name: 'Continue' });
    await expect(continueButton).toBeDisabled();
    await page.getByTestId('edge-deploy-confirmed').click();
    await expect(continueButton).toBeEnabled();
    await continueButton.click();

    const nameField = page.getByTestId('field-common_name');
    await nameField.fill(EXISTING_EDGE_NAME);
    await page.getByTestId('field-ip_or_domain').fill('edge');
    await page.getByRole('button', { name: 'Adopt Edge component' }).click();

    await expect(page.locator('.field-error')).toHaveText(
      'An Edge Component with this name already exists',
    );
    await expect(nameField).toBeVisible();
    await expect(page.locator('.loading-step')).toHaveCount(0);
  });
});
