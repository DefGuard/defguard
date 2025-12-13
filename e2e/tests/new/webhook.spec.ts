import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../../config';
import { loginBasic } from '../../utils/controllers/login';
import { createWebhook } from '../../utils/controllers/webhook';
import { dockerRestart } from '../../utils/docker';
import { waitForPromise } from '../../utils/waitForPromise';

test.describe('Test webhooks', () => {
  test.beforeEach(() => {
    dockerRestart();
  });
  const webhook_url = 'https://defguard.defguard/webhook';
  const webhook_description = 'example webhook';
  const webhook_secret = 'secret';

  test('Create webhook and verify content', async ({ page, browser }) => {
    await createWebhook(browser, webhook_url, webhook_description, webhook_secret);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.webhooks, {
      waitUntil: 'networkidle',
    });
    const webhookRow = await page
      .locator('.virtual-row')
      .filter({ hasText: webhook_url });
    await expect(webhookRow).toContainText(webhook_url);
    await expect(webhookRow).toContainText(webhook_description);
    await expect(webhookRow).toContainText('Active');
  });

  const new_webhook_url = 'https://changed.defguard/webhook';
  const new_webhook_description = 'changed webhook';

  test('Create, modify webhook and verify content', async ({ page, browser }) => {
    await createWebhook(browser, webhook_url, webhook_description, 'secret');
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.webhooks, {
      waitUntil: 'networkidle',
    });
    const webhookRow = await page
      .locator('.virtual-row')
      .filter({ hasText: webhook_url });
    await expect(webhookRow).toContainText(webhook_url);
    await expect(webhookRow).toContainText(webhook_description);
    await expect(webhookRow).toContainText('Active');
    // check if webhook is OK
    // then edit webhook
    await webhookRow.locator('.icon-button').click();
    await page.getByTestId('edit').click();

    await page.getByTestId('field-url').fill(new_webhook_url);
    await page.getByTestId('field-description').fill(new_webhook_description);
    await page.getByTestId('submit').click();
    await waitForPromise(2000);
    await page.goto(routes.base + routes.webhooks, {
      waitUntil: 'networkidle',
    });

    const new_webhookRow = await page
      .locator('.virtual-row')
      .filter({ hasText: new_webhook_url });
    await expect(new_webhookRow).toContainText(new_webhook_url);
    await expect(new_webhookRow).toContainText(new_webhook_description);
    await expect(new_webhookRow).toContainText('Active');
  });

  test('Create webhook, change state and verify content', async ({ page, browser }) => {
    await createWebhook(browser, webhook_url, webhook_description, webhook_secret);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.webhooks, {
      waitUntil: 'networkidle',
    });
    const webhookRow = await page
      .locator('.virtual-row')
      .filter({ hasText: webhook_url });
    await expect(webhookRow).toContainText(webhook_url);
    await expect(webhookRow).toContainText(webhook_description);
    await expect(webhookRow).toContainText('Active');

    // is everything ok after changing state to Disabled?
    await webhookRow.locator('.icon-button').click();
    await page.getByTestId('change-state').click();
    await page.locator('.virtual-row').filter({ hasText: webhook_url });
    await expect(webhookRow).toContainText(webhook_url);
    await expect(webhookRow).toContainText(webhook_description);
    await expect(webhookRow).toContainText('Disabled');

    // is everything ok after changing state to Enabled?
    await webhookRow.locator('.icon-button').click();
    await page.getByTestId('change-state').click();
    await expect(webhookRow).toContainText(webhook_url);
    await expect(webhookRow).toContainText(webhook_description);
    await expect(webhookRow).toContainText('Active');
  });
  test('Create webhook and delete it', async ({ page, browser }) => {
    await createWebhook(browser, webhook_url, webhook_description, webhook_secret);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.webhooks, {
      waitUntil: 'networkidle',
    });
    const webhookRow = await page
      .locator('.virtual-row')
      .filter({ hasText: webhook_url });
    await expect(webhookRow).toContainText(webhook_url);
    await expect(webhookRow).toContainText(webhook_description);
    await expect(webhookRow).toContainText('Active');
    await webhookRow.locator('.icon-button').click();
    await page.getByTestId('delete').click();

    await expect(webhookRow).not.toBeVisible();
  });
});
