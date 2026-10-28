import { APIRequestContext, expect, test } from '@playwright/test';
import { TOTP } from 'totp-generator';

import { defaultUserAdmin, testsConfig, testUserTemplate } from '../config';
import { User } from '../types';
import {
  apiCreateDevice,
  apiCreateMfaLocation,
  clientMfaConnect,
  clientMfaFinish,
  clientMfaFinishResult,
  clientMfaStartStep,
  MfaMethod,
  waitForProxy,
} from '../utils/api/clientMfa';
import { apiLogin } from '../utils/api/users';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic } from '../utils/controllers/login';
import { enableEmailMFA } from '../utils/controllers/mfa/enableEmail';
import { enableTOTP } from '../utils/controllers/mfa/enableTOTP';
import {
  addMfaFlowStep,
  changeMfaFlowStepMethod,
  createMfaFlow,
  deleteMfaFlow,
  openMfaFlowEdit,
  reorderMfaFlowSteps,
  saveMfaFlow,
} from '../utils/controllers/mfaFlow';
import { setupSMTP } from '../utils/controllers/settings';
import { dockerRestart } from '../utils/docker';

const EMAIL_CODE_PERIOD = 300;
const coreUrl = (path: string) => testsConfig.CORE_BASE_URL + path;

const getFlowId = async (request: APIRequestContext, title: string) => {
  const response = await request.get(coreUrl('/mfa-flow'));
  expect(response.status()).toBe(200);
  const flows: { id: number; title: string; step_count: number }[] =
    await response.json();
  const flow = flows.find((item) => item.title === title);
  expect(flow).toBeDefined();
  if (!flow) throw new Error(`MFA flow ${title} was not found.`);
  return flow.id;
};

test.describe('MFA flows', () => {
  test.beforeEach(() => {
    dockerRestart();
  });

  test('Create, edit, reorder, list, and delete a flow', async ({
    page,
    browser,
    request,
  }) => {
    const title = 'e2e-flow-crud';
    await setupSMTP(browser);
    await loginBasic(page, defaultUserAdmin);
    expect(
      await apiLogin(request, defaultUserAdmin.username, defaultUserAdmin.password),
    ).toBe(200);
    await createMfaFlow(page, title, [['totp']]);

    const flowId = await getFlowId(request, title);
    const initialResponse = await request.get(coreUrl(`/mfa-flow/${flowId}`));
    expect(initialResponse.status()).toBe(200);
    expect((await initialResponse.json()).steps.map((step) => step.methods)).toEqual([
      ['totp'],
    ]);

    const row = page.getByTestId(`mfa-flow-row-${flowId}`);
    await expect(row).toBeVisible();
    await expect(row).toContainText(title);

    await openMfaFlowEdit(page, flowId);
    await addMfaFlowStep(page, 'email');
    await changeMfaFlowStepMethod(page, 1, 'totp', 'email');
    await changeMfaFlowStepMethod(page, 2, 'email', 'totp');
    await expect(page.getByTestId('remove-mfa-method-1-email')).toBeVisible();
    await expect(page.getByTestId('remove-mfa-method-2-totp')).toBeVisible();
    await reorderMfaFlowSteps(page, 2, 1);
    await expect(page.getByTestId('remove-mfa-method-1-totp')).toBeVisible();
    await expect(page.getByTestId('remove-mfa-method-2-email')).toBeVisible();
    await saveMfaFlow(page);

    const updatedResponse = await request.get(coreUrl(`/mfa-flow/${flowId}`));
    expect(updatedResponse.status()).toBe(200);
    expect((await updatedResponse.json()).steps.map((step) => step.methods)).toEqual([
      ['totp'],
      ['email'],
    ]);
    await expect(row).toBeVisible();

    const listResponse = await request.get(coreUrl('/mfa-flow'));
    expect(listResponse.status()).toBe(200);
    const listedFlows: { id: number; title: string; step_count: number }[] =
      await listResponse.json();
    expect(listedFlows.find((flow) => flow.id === flowId)?.step_count).toBe(2);

    await deleteMfaFlow(page, flowId);
    await expect(row).not.toBeVisible();
    const deletedResponse = await request.get(coreUrl(`/mfa-flow/${flowId}`));
    expect(deletedResponse.status()).toBe(404);
    const remainingFlows: { id: number }[] = await (
      await request.get(coreUrl('/mfa-flow'))
    ).json();
    expect(remainingFlows.some((flow) => flow.id === flowId)).toBe(false);
  });

  test('Enforce a TOTP then Email flow over Edge', async ({ page, browser, request }) => {
    const testUser: User = { ...testUserTemplate, username: 'mfa-flow-user' };
    await createUser(browser, testUser);
    const { secret: totpSecret } = await enableTOTP(browser, testUser);
    const { secret: emailSecret } = await enableEmailMFA(browser, testUser, totpSecret);
    expect(
      await apiLogin(request, defaultUserAdmin.username, defaultUserAdmin.password),
    ).toBe(200);
    await waitForProxy(request);
    await loginBasic(page, defaultUserAdmin);

    const title = 'e2e-totp-email-flow';
    await createMfaFlow(page, title, [['totp'], ['email']]);
    const flowId = await getFlowId(request, title);
    const locationId = await apiCreateMfaLocation(request, flowId);
    const pubkey = await apiCreateDevice(request, testUser.username, 'mfa-flow-device');
    const device = {
      locationId,
      pubkey,
      protocol: 'multi-step' as const,
      method: MfaMethod.TOTP,
    };
    const attempt = await clientMfaConnect(request, device, [
      MfaMethod.TOTP,
      MfaMethod.EMAIL,
    ]);

    const { otp: totpCode } = await TOTP.generate(totpSecret);
    const firstResponse = await clientMfaFinish(request, attempt, totpCode);
    const firstResult = await clientMfaFinishResult(firstResponse);
    expect(firstResult.kind).toBe('Advanced');
    if (firstResult.kind !== 'Advanced') throw new Error('MFA flow did not advance.');
    expect(firstResult.nextStep).toBe(1);
    expect(firstResult.presharedKey).toBeUndefined();

    const stepAttemptId = await clientMfaStartStep(
      request,
      attempt.token,
      MfaMethod.EMAIL,
    );
    const { otp: emailCode } = await TOTP.generate(emailSecret, {
      digits: 6,
      period: EMAIL_CODE_PERIOD,
    });
    const secondResponse = await clientMfaFinish(
      request,
      { token: attempt.token, stepAttemptId },
      emailCode,
    );
    const secondResult = await clientMfaFinishResult(secondResponse);
    expect(secondResult.kind).toBe('Completed');
    if (secondResult.kind !== 'Completed') throw new Error('MFA flow did not complete.');
    expect(secondResult.presharedKey).toBeTruthy();
  });
});
