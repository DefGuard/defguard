import { Page } from 'playwright';

import { routes } from '../../config';

export type MfaFlowMethod = 'totp' | 'email';
export type MfaFlowSteps = MfaFlowMethod[][];

export const createMfaFlow = async (
  page: Page,
  title: string,
  steps: MfaFlowSteps,
): Promise<void> => {
  await page.goto(routes.base + routes.mfa.list);
  await page.getByTestId('add-mfa-flow').click();
  await page.getByTestId('field-title').fill(title);

  for (const [index, methods] of steps.entries()) {
    const [firstMethod, ...additionalMethods] = methods;
    if (!firstMethod) throw new Error('Each MFA flow step needs a method.');

    await page.getByTestId('add-mfa-step').click();
    await page.getByTestId(`add-mfa-step-method-${firstMethod}`).click();
    for (const method of additionalMethods) {
      await page.getByTestId(`add-mfa-method-${index + 1}`).click();
      await page.getByTestId(`add-mfa-method-${index + 1}-${method}`).click();
    }
  }

  await page.getByTestId('save-mfa-flow').click();
  await page.waitForURL(`**${routes.mfa.list}`);
};

export const openMfaFlowEdit = async (page: Page, flowId: number): Promise<void> => {
  await page.getByTestId(`mfa-flow-actions-${flowId}`).click();
  await page.getByTestId(`edit-mfa-flow-${flowId}`).click();
  await page.waitForURL(`**${routes.mfa.editFlow(flowId)}`);
};

const saveLocationMfaChanges = async (
  page: Page,
  locationId: number,
  requiresConfirmation: boolean,
): Promise<void> => {
  const saveResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'PUT' &&
      response.url().endsWith(`/network/${locationId}`),
  );
  await page.getByTestId('save-location').click();
  if (requiresConfirmation) {
    await page.getByRole('button', { name: 'Save changes anyway' }).click();
  }
  const response = await saveResponse;
  if (!response.ok()) throw new Error(`Saving location failed: ${response.status()}`);
};

export const assignDefaultLocationMfaFlow = async (
  page: Page,
  locationId: number,
  flowId: number,
): Promise<void> => {
  await page.goto(`${routes.base}${routes.locations}/${locationId}/edit`);
  await page.getByTestId('location-mfa-enabled-toggle').click();
  await page.getByTestId(`location-mfa-flow-option-${flowId}`).click();
  await page.getByTestId('location-mfa-assignment-save').click();
  await saveLocationMfaChanges(page, locationId, true);
};

export const addLocationMfaOverride = async (
  page: Page,
  locationId: number,
  flowId: number,
  groupId: number,
): Promise<void> => {
  await page.goto(`${routes.base}${routes.locations}/${locationId}/edit`);
  await page.getByTestId('location-mfa-add-override').click();
  await page.getByTestId(`location-mfa-flow-option-${flowId}`).click();
  await page.getByTestId('location-mfa-assignment-continue').click();
  await page.getByTestId(`location-mfa-override-group-${groupId}`).click();
  await page.getByTestId('location-mfa-assignment-save').click();
  await saveLocationMfaChanges(page, locationId, false);
};

export const addMfaFlowStep = async (
  page: Page,
  method: MfaFlowMethod,
): Promise<void> => {
  await page.getByTestId('add-mfa-step').click();
  await page.getByTestId(`add-mfa-step-method-${method}`).click();
};

export const changeMfaFlowStepMethod = async (
  page: Page,
  stepNumber: number,
  currentMethod: MfaFlowMethod,
  nextMethod: MfaFlowMethod,
): Promise<void> => {
  await page.getByTestId(`remove-mfa-method-${stepNumber}-${currentMethod}`).click();
  await page.getByTestId(`add-mfa-method-${stepNumber}`).click();
  await page.getByTestId(`add-mfa-method-${stepNumber}-${nextMethod}`).click();
};

export const reorderMfaFlowSteps = async (
  page: Page,
  fromStep: number,
  toStep: number,
): Promise<void> => {
  const handle = page.getByTestId(`reorder-mfa-step-${fromStep}`);
  const target = page.getByTestId(`mfa-flow-step-${toStep}`);
  const handleBox = await handle.boundingBox();
  const targetBox = await target.boundingBox();
  if (!handleBox || !targetBox) throw new Error('Unable to locate MFA flow steps.');

  await page.mouse.move(
    handleBox.x + handleBox.width / 2,
    handleBox.y + handleBox.height / 2,
  );
  await page.mouse.down();
  await page.mouse.move(
    targetBox.x + targetBox.width / 2,
    targetBox.y + targetBox.height / 2,
    { steps: 10 },
  );
  await page.mouse.up();
};

export const saveMfaFlow = async (page: Page): Promise<void> => {
  await page.getByTestId('save-mfa-flow').click();
  await page.waitForURL(`**${routes.mfa.list}`);
};

export const deleteMfaFlow = async (page: Page, flowId: number): Promise<void> => {
  await page.getByTestId(`mfa-flow-actions-${flowId}`).click();
  await page.getByTestId(`delete-mfa-flow-${flowId}`).click();
  await page.getByTestId('confirm-delete-mfa-flow').click();
};
