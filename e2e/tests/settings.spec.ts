import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { NetworkForm, User } from '../types';
import { createUser } from '../utils/controllers/createUser';
import { createGroup } from '../utils/controllers/groups';
import { loginBasic } from '../utils/controllers/login';
import { setupSMTP } from '../utils/controllers/settings';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

const testNetwork: NetworkForm = {
  name: 'settings location',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  port: '5055',
};

test.describe('Settings', () => {
  test.beforeEach(() => dockerRestart());

  test('Instance name, public Edge URL and session duration are saved', async ({
    page,
  }) => {
    const instanceName = 'E2E instance';
    const publicEdgeUrl = 'http://edge.e2e.test';
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.settings.instance);

    await page.getByTestId('field-instance_name').fill(instanceName);
    await page.getByTestId('field-public_proxy_url').fill(publicEdgeUrl);
    await page.getByTestId('field-authentication_period_days').click();
    await page.locator('.select-floating .select-option', { hasText: '14 days' }).click();
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page.getByText('Settings saved')).toBeVisible();

    await page.reload();
    await expect(page.getByTestId('field-instance_name')).toHaveValue(instanceName);
    await expect(page.getByTestId('field-public_proxy_url')).toHaveValue(publicEdgeUrl);
    await expect(page.getByTestId('field-authentication_period_days')).toContainText(
      '14 days',
    );
  });

  test.describe('Client behavior', () => {
    test.skip(!process.env.DEFGUARD_LICENSE_KEY, 'Requires DEFGUARD_LICENSE_KEY.');

    test('Client permissions and traffic policy are saved', async ({ page }) => {
      await waitForBase(page);
      await loginBasic(page, defaultUserAdmin);
      await page.goto(routes.base + routes.settings.client);

      const deviceManagement = page.locator('.interactive-block', {
        hasText: 'Device management for users',
      });
      const wireguardConfig = page.locator('.interactive-block', {
        hasText: 'WireGuard configuration for users',
      });
      const forceAllTraffic = page
        .locator('.interactive-block.variant-radio', { hasText: 'Force all traffic' })
        .first();
      await expect(deviceManagement.locator('.toggle')).toHaveAttribute(
        'data-value',
        'false',
      );
      await expect(wireguardConfig.locator('.toggle')).toHaveAttribute(
        'data-value',
        'false',
      );
      await expect(forceAllTraffic.locator('.radio .inner')).toHaveAttribute(
        'data-active',
        'false',
      );

      await deviceManagement.locator('.interactive').click();
      await wireguardConfig.locator('.interactive').click();
      await forceAllTraffic.locator('.interactive').click();
      await page.getByRole('button', { name: 'Save changes', exact: true }).click();
      await expect(page.getByText('Settings saved')).toBeVisible();

      await page.reload();
      await expect(deviceManagement.locator('.toggle')).toHaveAttribute(
        'data-value',
        'true',
      );
      await expect(wireguardConfig.locator('.toggle')).toHaveAttribute(
        'data-value',
        'true',
      );
      await expect(forceAllTraffic.locator('.radio .inner')).toHaveAttribute(
        'data-active',
        'true',
      );
    });

    test('Group-based traffic policies are saved', async ({ page, browser }) => {
      const forceGroup = 'force-traffic';
      const disableGroup = 'disable-traffic';
      await waitForBase(page);
      await createGroup(browser, forceGroup);
      await createGroup(browser, disableGroup);
      await loginBasic(page, defaultUserAdmin);
      await page.goto(routes.base + routes.settings.client);

      const forceRow = page.locator('.group-policy-row', {
        hasText: 'Force all traffic',
      });
      const disableRow = page.locator('.group-policy-row', {
        hasText: 'Disable all traffic',
      });
      const selectionModal = page.locator('#selection-modal');
      await forceRow.getByRole('button', { name: 'Edit groups' }).click();
      await selectionModal
        .locator('.item', { hasText: forceGroup })
        .locator('.checkbox')
        .click();
      await selectionModal.getByRole('button', { name: 'Submit', exact: true }).click();
      await selectionModal.waitFor({ state: 'hidden' });

      await disableRow.getByRole('button', { name: 'Edit groups' }).click();
      await expect(selectionModal.locator('.item', { hasText: forceGroup })).toHaveCount(
        0,
      );
      await selectionModal
        .locator('.item', { hasText: disableGroup })
        .locator('.checkbox')
        .click();
      await selectionModal.getByRole('button', { name: 'Submit', exact: true }).click();
      await selectionModal.waitFor({ state: 'hidden' });
      await page.getByRole('button', { name: 'Save changes', exact: true }).click();
      await expect(page.getByText('Settings saved')).toBeVisible();

      await page.reload();
      await expect(forceRow.locator('.chip')).toHaveText([forceGroup]);
      await expect(disableRow.locator('.chip')).toHaveText([disableGroup]);
    });

    test('Device management for users blocks adding devices', async ({
      page,
      browser,
    }) => {
      const testUser: User = { ...testUserTemplate, username: 'test' };
      const devicesUrl =
        routes.base + routes.profile + testUser.username + routes.tab.devices;
      await waitForBase(page);
      await createRegularLocation(browser, testNetwork);
      await createUser(browser, testUser);

      const userContext = await browser.newContext();
      const userPage = await userContext.newPage();
      await loginBasic(userPage, testUser);
      await userPage.goto(devicesUrl);
      await expect(userPage.getByTestId('add-device')).toBeEnabled();

      await loginBasic(page, defaultUserAdmin);
      await page.goto(routes.base + routes.settings.client);
      await page
        .locator('.interactive-block', { hasText: 'Device management for users' })
        .locator('.interactive')
        .click();
      await page.getByRole('button', { name: 'Save changes', exact: true }).click();
      await expect(page.getByText('Settings saved')).toBeVisible();

      await userPage.reload();
      await expect(userPage.getByTestId('add-device')).toBeDisabled();
      await userContext.close();
    });
  });

  test('SMTP configuration is saved', async ({ page, browser }) => {
    await waitForBase(page);
    await setupSMTP(browser);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.settings.smtp);

    const card = page.getByTestId('smtp-auth-method-card-none');
    await expect(card).toContainText('Applied');
    await card.locator('.icon-button').click();
    await page.getByTestId('smtp-card-none-edit').click();
    await expect(page.getByTestId('field-smtp_server')).toHaveValue('mailpit');
    await expect(page.getByTestId('field-smtp_port')).toHaveValue('1025');
    await expect(page.getByTestId('field-smtp_sender')).toHaveValue('test@test.com');
  });

  test('SMTP configuration is deleted', async ({ page, browser }) => {
    await waitForBase(page);
    await setupSMTP(browser);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.settings.smtp);

    const card = page.getByTestId('smtp-auth-method-card-none');
    await card.locator('.icon-button').click();
    await page.getByTestId('smtp-card-none-delete').click();
    await page.getByTestId('confirm-action-submit').click();
    await expect(page.getByText('SMTP settings reset')).toBeVisible();

    await page.reload();
    await expect(page.getByTestId('smtp-card-none-configure')).toBeVisible();
    await expect(card).not.toContainText('Applied');
  });

  test('SMTP deletion is blocked by gateway notifications', async ({ page, browser }) => {
    await waitForBase(page);
    await setupSMTP(browser);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.settings.gatewayNotifications);
    await page
      .locator('.interactive-block', { hasText: 'Gateway disconnect notifications' })
      .locator('.interactive')
      .first()
      .click();
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page.getByText('Settings saved')).toBeVisible();

    await page.goto(routes.base + routes.settings.smtp);
    const card = page.getByTestId('smtp-auth-method-card-none');
    await card.locator('.icon-button').click();
    await page.getByTestId('smtp-card-none-delete').click();
    const modal = page.locator('#action-modal');
    await expect(modal).toContainText(
      'SMTP settings cannot be reset while these features are enabled',
    );
    await expect(modal).toContainText('Gateway disconnect notifications');
    await expect(modal.getByTestId('confirm-action-submit')).toHaveCount(0);
    await modal.getByTestId('confirm-action-cancel').click();
    await modal.waitFor({ state: 'hidden' });

    await page.reload();
    await expect(card).toContainText('Applied');
  });

  test('Gateway notifications require SMTP', async ({ page }) => {
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.settings.gatewayNotifications);

    await expect(
      page.getByText('To enable notifications you must first configure SMTP.'),
    ).toBeVisible();
    await expect(
      page.locator('.interactive-block', { hasText: 'Gateway disconnect notifications' }),
    ).toHaveClass(/\bdisabled\b/);
    await expect(
      page.locator('.interactive-block', { hasText: 'Gateway reconnect notifications' }),
    ).toHaveClass(/\bdisabled\b/);
  });

  test('Gateway notifications are saved', async ({ page, browser }) => {
    await waitForBase(page);
    await setupSMTP(browser);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.settings.gatewayNotifications);

    const disconnect = page.locator('.interactive-block', {
      hasText: 'Gateway disconnect notifications',
    });
    const reconnect = page.locator('.interactive-block', {
      hasText: 'Gateway reconnect notifications',
    });
    const threshold = page.getByTestId(
      'field-gateway_disconnect_notifications_inactivity_threshold',
    );
    await expect(disconnect.locator('.toggle')).toHaveAttribute('data-value', 'false');
    await expect(reconnect.locator('.toggle')).toHaveAttribute('data-value', 'false');

    await disconnect.locator('.interactive').first().click();
    await threshold.fill('15');
    await reconnect.locator('.interactive').click();
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page.getByText('Settings saved')).toBeVisible();

    await page.reload();
    await expect(disconnect.locator('.toggle')).toHaveAttribute('data-value', 'true');
    await expect(reconnect.locator('.toggle')).toHaveAttribute('data-value', 'true');
    await expect(threshold).toHaveValue('15');
  });
});
