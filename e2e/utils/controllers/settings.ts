import { Browser, expect, Page } from '@playwright/test';

import { defaultUserAdmin, routes } from '../../config';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';
import { logout } from './logout';

// Configure SMTP in Core settings as an admin.
// Several Edge features (password reset, email MFA) are only exposed once SMTP
// is configured, so tests that exercise them must call this first.
export const setupSMTP = async (browser: Browser) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.settings.smtp);
  await page.getByTestId('smtp-card-none-configure').click();
  await page.getByTestId('field-smtp_server').waitFor({ state: 'visible' });
  await page.getByTestId('field-smtp_server').fill('mailpit');
  await page.getByTestId('field-smtp_port').fill('1025');
  await page.getByTestId('field-smtp_sender').fill('test@test.com');
  await page.getByTestId('field-smtp_encryption').click();
  await page.locator('.select-floating [role="listitem"]').first().click();
  const saveButton = await page.getByTestId('submit');
  if (await saveButton.isEnabled()) {
    await saveButton.click();
  }
  await logout(page);
  await context.close();
};

export type LogStream = {
  type: 'Vector' | 'Logstash';
  name: string;
  url: string;
};

export const addLogStream = async (browser: Browser, stream: LogStream) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.settings.activityStreaming);
  await page.getByTestId('add-activity-stream').click();
  await page.getByTestId(`add-${stream.type.toLowerCase()}`).click();
  await page.getByTestId('field-name').fill(stream.name);
  await page.getByTestId('field-url').fill(stream.url);
  await page.getByTestId('add-destination-submit').click();
  await expect(
    page.locator('.virtual-row').filter({ hasText: stream.name }),
  ).toBeVisible();
  await context.close();
};

export const licenseProperty = (page: Page, title: string) =>
  page.locator('.license-property-info', { hasText: title }).locator('.bottom');
