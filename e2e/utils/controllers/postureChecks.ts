import { Browser, expect, Locator, Page } from '@playwright/test';

import { defaultUserAdmin, routes } from '../../config';
import { waitForBase } from '../waitForBase';
import { waitForRoute } from '../waitForRoute';
import { loginBasic } from './login';

export type PostureCheckSystem = {
  os: 'Windows' | 'macOS' | 'Linux' | 'iOS' | 'Android';
  version?: string;
  period?: string;
  conditions?: string[];
};

export type PostureCheck = {
  name: string;
  description?: string;
  systems: PostureCheckSystem[];
  desktopVersion?: string;
  mobileVersion?: string;
  prerelease?: boolean;
  desktopAttestation?: boolean;
  desktopVerification?: boolean;
  mobileAttestation?: boolean;
};

export const selectPostureOption = async (
  page: Page,
  select: Locator,
  option: string,
) => {
  await select.click();
  await page.locator('.select-floating .select-option', { hasText: option }).click();
};

export const addPostureOs = async (page: Page, system: PostureCheckSystem) => {
  await page
    .locator('.posture-check-operating-systems .selectors')
    .getByText(system.os, { exact: true })
    .click();
  const card = page.locator('.policy-os-card', { hasText: system.os });
  if (system.version) {
    await selectPostureOption(page, card.locator('.select').nth(0), system.version);
  }
  if (system.period) {
    await selectPostureOption(page, card.locator('.select').nth(1), system.period);
  }
  for (const condition of system.conditions ?? []) {
    await card.locator('.checkbox', { hasText: condition }).click();
  }
};

export const fillPostureCheckWizard = async (page: Page, postureCheck: PostureCheck) => {
  await page.goto(routes.base + routes.postureChecks);
  await page.getByRole('button', { name: 'Add new posture check' }).click();
  for (const system of postureCheck.systems) {
    await addPostureOs(page, system);
  }
  const continueButton = page.getByRole('button', { name: 'Continue' });
  await continueButton.click();

  const clientSelects = page.locator('.add-posture-check-client-version-step .select');
  if (postureCheck.desktopVersion) {
    await selectPostureOption(page, clientSelects.nth(0), postureCheck.desktopVersion);
  }
  if (postureCheck.mobileVersion) {
    await selectPostureOption(page, clientSelects.nth(1), postureCheck.mobileVersion);
  }
  if (postureCheck.prerelease) {
    await page
      .locator('.interactive-block', { hasText: 'pre-release' })
      .locator('.interactive')
      .click();
  }
  await continueButton.click();

  const attestation = page.locator('.add-posture-check-device-attestation-step');
  if (postureCheck.desktopAttestation) {
    await attestation
      .locator('.interactive-block', { hasText: 'Desktop Device Attestation' })
      .locator('.interactive')
      .click();
  }
  if (postureCheck.desktopVerification) {
    await attestation
      .locator('.interactive-block.variant-checkbox')
      .nth(0)
      .locator('.interactive')
      .click();
  }
  if (postureCheck.mobileAttestation) {
    await attestation
      .locator('.interactive-block', { hasText: 'Mobile Device Attestation' })
      .locator('.interactive')
      .click();
  }
  await continueButton.click();

  await page.getByRole('textbox', { name: 'Name' }).fill(postureCheck.name);
  if (postureCheck.description) {
    await page
      .getByRole('textbox', { name: 'Description' })
      .fill(postureCheck.description);
  }
  await continueButton.click();
};

export const confirmPostureCheckCreation = async (page: Page) => {
  await page.getByRole('button', { name: 'Create posture check' }).click();
  const modal = page.locator('#action-modal');
  await expect(modal).toContainText('Prevent admin lockout');
  await modal.getByTestId('confirm-action-submit').click();
  await waitForRoute(page, routes.postureChecks);
};

export const createPostureCheck = async (
  browser: Browser,
  postureCheck: PostureCheck,
) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await fillPostureCheckWizard(page, postureCheck);
  await confirmPostureCheckCreation(page);
  await expect(
    page.locator('.virtual-row').filter({ hasText: postureCheck.name }),
  ).toBeVisible();
  await context.close();
};
