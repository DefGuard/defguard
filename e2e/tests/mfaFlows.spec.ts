import { APIRequestContext, expect, test } from '@playwright/test';
import { TOTP } from 'totp-generator';

import { defaultUserAdmin, testsConfig, testUserTemplate } from '../config';
import { NetworkForm, User } from '../types';
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
import { createGroup } from '../utils/controllers/groups';
import { loginBasic } from '../utils/controllers/login';
import { enableEmailMFA } from '../utils/controllers/mfa/enableEmail';
import { enableTOTP } from '../utils/controllers/mfa/enableTOTP';
import {
  addLocationMfaOverride,
  addMfaFlowStep,
  assignDefaultLocationMfaFlow,
  changeMfaFlowStepMethod,
  createMfaFlow,
  deleteMfaFlow,
  openMfaFlowEdit,
  reorderMfaFlowSteps,
  saveMfaFlow,
} from '../utils/controllers/mfaFlow';
import { setupSMTP } from '../utils/controllers/settings';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
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

type LocationMfaAssignment = {
  title: string;
  is_default: boolean;
  groups: { id: number; name: string }[];
};

const getLocationId = async (request: APIRequestContext, name: string) => {
  const response = await request.get(coreUrl('/network'));
  expect(response.status()).toBe(200);
  const locations: { id: number; name: string }[] = await response.json();
  const location = locations.find((item) => item.name === name);
  expect(location).toBeDefined();
  if (!location) throw new Error(`Location ${name} was not found.`);
  return location.id;
};

const getGroupId = async (request: APIRequestContext, name: string) => {
  const response = await request.get(coreUrl('/group-info'));
  expect(response.status()).toBe(200);
  const groups: { id: number; name: string }[] = await response.json();
  const group = groups.find((item) => item.name === name);
  expect(group).toBeDefined();
  if (!group) throw new Error(`Group ${name} was not found.`);
  return group.id;
};

const getLocationAssignments = async (
  request: APIRequestContext,
  locationId: number,
): Promise<LocationMfaAssignment[]> => {
  const response = await request.get(coreUrl(`/location/${locationId}/mfa-flows`));
  expect(response.status()).toBe(200);
  return response.json();
};

const expectDefaultFlowAssignment = async (
  request: APIRequestContext,
  locationId: number,
  title: string,
) => {
  const assignments = await getLocationAssignments(request, locationId);
  expect(assignments).toContainEqual(
    expect.objectContaining({ title, is_default: true, groups: [] }),
  );
};

const enforceTOTPFlowOverEdge = async (
  request: APIRequestContext,
  username: string,
  locationId: number,
  totpSecret: string,
) => {
  const pubkey = await apiCreateDevice(request, username, `${username}-device`);
  const attempt = await clientMfaConnect(
    request,
    { locationId, pubkey, protocol: 'multi-step', method: MfaMethod.TOTP },
    [MfaMethod.TOTP],
  );
  const { otp } = await TOTP.generate(totpSecret);
  const result = await clientMfaFinishResult(
    await clientMfaFinish(request, attempt, otp),
  );
  expect(result.kind).toBe('Completed');
  if (result.kind !== 'Completed')
    throw new Error('Assigned TOTP flow did not complete.');
  expect(result.presharedKey).toBeTruthy();
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

  test('Assign a default flow on the edit page and enforce it over Edge', async ({
    page,
    browser,
    request,
  }) => {
    const testUser: User = { ...testUserTemplate, username: 'mfa-edit-flow-user' };
    await createUser(browser, testUser);
    const { secret } = await enableTOTP(browser, testUser);
    expect(
      await apiLogin(request, defaultUserAdmin.username, defaultUserAdmin.password),
    ).toBe(200);
    await waitForProxy(request);
    await loginBasic(page, defaultUserAdmin);

    const title = 'e2e-edit-location-totp-flow';
    await createMfaFlow(page, title, [['totp']]);
    const flowId = await getFlowId(request, title);
    const network: NetworkForm = {
      name: 'e2e-edit-flow-location',
      address: '10.55.0.1/24',
      endpoint: '192.168.4.15',
      port: '51831',
      allowed_ips: ['10.55.0.0/24'],
    };
    await createRegularLocation(browser, network);
    const locationId = await getLocationId(request, network.name);
    await assignDefaultLocationMfaFlow(page, locationId, flowId);

    await expectDefaultFlowAssignment(request, locationId, title);
    await enforceTOTPFlowOverEdge(request, testUser.username, locationId, secret);
  });

  test('Assign a default flow in the wizard and enforce it over Edge', async ({
    page,
    browser,
    request,
  }) => {
    const testUser: User = { ...testUserTemplate, username: 'mfa-wizard-flow-user' };
    await createUser(browser, testUser);
    const { secret } = await enableTOTP(browser, testUser);
    expect(
      await apiLogin(request, defaultUserAdmin.username, defaultUserAdmin.password),
    ).toBe(200);
    await waitForProxy(request);
    await loginBasic(page, defaultUserAdmin);

    const title = 'e2e-wizard-location-totp-flow';
    await createMfaFlow(page, title, [['totp']]);
    const flowId = await getFlowId(request, title);
    const network: NetworkForm = {
      name: 'e2e-wizard-flow-location',
      address: '10.56.0.1/24',
      endpoint: '192.168.4.16',
      port: '51832',
      allowed_ips: ['10.56.0.0/24'],
      mfa_enabled: true,
      defaultMfaFlowId: flowId,
    };
    await createRegularLocation(browser, network);
    const locationId = await getLocationId(request, network.name);

    await expectDefaultFlowAssignment(request, locationId, title);
    await enforceTOTPFlowOverEdge(request, testUser.username, locationId, secret);
  });

  test('Apply a group override on Edge and fall back to the default flow', async ({
    page,
    browser,
    request,
  }) => {
    test.skip(!process.env.DEFGUARD_LICENSE_KEY, 'Requires an Enterprise license.');

    const groupName = 'e2e-mfa-override-group';
    const member: User = {
      ...testUserTemplate,
      username: 'mfa-override-member',
      mail: 'mfa-override-member@test.com',
      phone: '123456780',
    };
    const nonMember: User = {
      ...testUserTemplate,
      username: 'mfa-override-non-member',
      mail: 'mfa-override-non-member@test.com',
      phone: '123456781',
    };
    await createGroup(browser, groupName);
    await createUser(browser, member, [groupName]);
    await createUser(browser, nonMember);
    const { secret: memberTotp } = await enableTOTP(browser, member);
    await enableEmailMFA(browser, member, memberTotp);
    await enableTOTP(browser, nonMember);

    expect(
      await apiLogin(request, defaultUserAdmin.username, defaultUserAdmin.password),
    ).toBe(200);
    await waitForProxy(request);
    await loginBasic(page, defaultUserAdmin);

    const defaultTitle = 'e2e-group-default-totp-flow';
    const overrideTitle = 'e2e-group-override-email-flow';
    await createMfaFlow(page, defaultTitle, [['totp']]);
    const defaultFlowId = await getFlowId(request, defaultTitle);
    await createMfaFlow(page, overrideTitle, [['email']]);
    const overrideFlowId = await getFlowId(request, overrideTitle);

    const network: NetworkForm = {
      name: 'e2e-group-override-location',
      address: '10.57.0.1/24',
      endpoint: '192.168.4.17',
      port: '51833',
      allowed_ips: ['10.57.0.0/24'],
      mfa_enabled: true,
      defaultMfaFlowId: defaultFlowId,
    };
    await createRegularLocation(browser, network);
    const locationId = await getLocationId(request, network.name);
    const groupId = await getGroupId(request, groupName);
    await addLocationMfaOverride(page, locationId, overrideFlowId, groupId);

    const assignments = await getLocationAssignments(request, locationId);
    expect(assignments).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          title: defaultTitle,
          is_default: true,
          groups: [],
        }),
        expect.objectContaining({
          title: overrideTitle,
          is_default: false,
          groups: [{ id: groupId, name: groupName }],
        }),
      ]),
    );

    const memberPubkey = await apiCreateDevice(
      request,
      member.username,
      'mfa-override-member-device',
    );
    await clientMfaConnect(
      request,
      {
        locationId,
        pubkey: memberPubkey,
        protocol: 'multi-step',
        method: MfaMethod.EMAIL,
      },
      [MfaMethod.EMAIL],
    );

    const nonMemberPubkey = await apiCreateDevice(
      request,
      nonMember.username,
      'mfa-override-non-member-device',
    );
    await clientMfaConnect(
      request,
      {
        locationId,
        pubkey: nonMemberPubkey,
        protocol: 'multi-step',
        method: MfaMethod.TOTP,
      },
      [MfaMethod.TOTP],
    );
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
