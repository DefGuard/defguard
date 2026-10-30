import { APIRequestContext, APIResponse, expect, test } from '@playwright/test';
import { TOTP } from 'totp-generator';

import { defaultUserAdmin, routes, testsConfig, testUserTemplate } from '../config';
import { NetworkForm, User } from '../types';
import {
  apiCreateDevice,
  apiCreateMfaLocation,
  clientMfaConnect,
  clientMfaFinish,
  clientMfaFinishResult,
  clientMfaStart,
  clientMfaStartStep,
  MfaMethod,
  waitForProxy,
} from '../utils/api/clientMfa';
import { expectLicenseTier, setLicense } from '../utils/api/settings';
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
const MFA_START_REJECTION_METHOD_NOT_IN_STEP = 1;
const coreUrl = (path: string) => testsConfig.CORE_BASE_URL + path;

test.use({ trace: 'off' });

const businessLicenseKey = process.env.DEFGUARD_BUSINESS_LICENSE_KEY?.trim();
const enterpriseLicenseKey = process.env.DEFGUARD_LICENSE_KEY?.trim();
const seedLicenseKey = enterpriseLicenseKey || businessLicenseKey;
const seedLicenseTier = enterpriseLicenseKey ? 'Enterprise' : 'Business';

type MfaFlowMethodValue = 'totp' | 'email';
type MfaFlowAssignmentInput = {
  flow_id: number;
  is_default: boolean;
  group_ids: number[];
};

const createMfaFlowViaApi = async (
  request: APIRequestContext,
  title: string,
  steps: MfaFlowMethodValue[][],
): Promise<number> => {
  const response = await request.post(coreUrl('/mfa-flow'), {
    data: { title, steps: steps.map((methods) => ({ methods })) },
  });
  expect(response.status()).toBe(201);
  return (await response.json()).id;
};

const mfaLocationData = (mfa_flows: MfaFlowAssignmentInput[]) => ({
  name: 'mfa-location',
  address: '10.99.0.1/24',
  endpoint: '192.168.4.14',
  port: 51830,
  allowed_ips: '10.99.0.0/24',
  dns: null,
  mtu: 1420,
  fwmark: 0,
  allowed_ips_from_acl: false,
  allow_all_groups: true,
  allowed_groups: [],
  keepalive_interval: 25,
  peer_disconnect_threshold: 300,
  acl_enabled: false,
  acl_default_allow: false,
  mfa_enabled: true,
  service_location_mode: 'disabled',
  posture_checks: [],
  mfa_flows,
});

const expectLicenseError = async (
  response: APIResponse,
  field: string,
  code: string,
): Promise<void> => {
  expect(response.status()).toBe(403);
  const body: { error: string; fields: { field: string; code: string }[] } =
    await response.json();
  expect(body.error).toBe('license_required');
  expect(body.fields).toContainEqual({ field, code });
};

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

  test.describe('Enterprise tier boundaries', () => {
    test.skip(!enterpriseLicenseKey, 'Requires DEFGUARD_LICENSE_KEY.');

    test('Apply a group override on Edge and fall back to the default flow', async ({
      page,
      browser,
      request,
    }) => {
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
      const { secret: memberEmailSecret } = await enableEmailMFA(
        browser,
        member,
        memberTotp,
      );
      const { secret: nonMemberTotp } = await enableTOTP(browser, nonMember);

      expect(
        await apiLogin(request, defaultUserAdmin.username, defaultUserAdmin.password),
      ).toBe(200);
      await expectLicenseTier(request, 'Enterprise');
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
      const memberAttempt = await clientMfaConnect(
        request,
        {
          locationId,
          pubkey: memberPubkey,
          protocol: 'multi-step',
          method: MfaMethod.EMAIL,
        },
        [MfaMethod.EMAIL],
      );
      const { otp: memberEmailCode } = await TOTP.generate(memberEmailSecret, {
        digits: 6,
        period: EMAIL_CODE_PERIOD,
      });
      const memberResult = await clientMfaFinishResult(
        await clientMfaFinish(request, memberAttempt, memberEmailCode),
      );
      expect(memberResult.kind).toBe('Completed');
      if (memberResult.kind !== 'Completed')
        throw new Error('Group override Email flow did not complete.');
      expect(memberResult.presharedKey).toBeTruthy();

      const memberTotpPubkey = await apiCreateDevice(
        request,
        member.username,
        'mfa-override-member-totp-device',
      );
      const refusedTotpStart = await clientMfaStart(
        request,
        {
          locationId,
          pubkey: memberTotpPubkey,
          protocol: 'multi-step',
          method: MfaMethod.TOTP,
        },
        [MfaMethod.TOTP],
      );
      expect(refusedTotpStart.status()).toBe(200);
      const refusedTotpBody = await refusedTotpStart.json();
      expect(refusedTotpBody.outcome?.Rejected?.rejections).toEqual([
        { step: 0, reason: MFA_START_REJECTION_METHOD_NOT_IN_STEP },
      ]);

      await enforceTOTPFlowOverEdge(
        request,
        nonMember.username,
        locationId,
        nonMemberTotp,
      );
    });
  });

  test.describe('Free tier boundaries', () => {
    test('Free rejects multi-step and additional flows', async ({
      page,
      browser,
      request,
    }) => {
      await setupSMTP(browser);
      await setLicense(request, null, null);
      await loginBasic(page, defaultUserAdmin);
      await page.goto(routes.base + routes.mfa.addFlow);
      await page.getByTestId('field-title').fill('e2e-free-ui-multi-step');
      await page.getByTestId('add-mfa-step').click();
      await page.getByTestId('add-mfa-step-method-totp').click();
      await expect(page.getByTestId('add-mfa-step')).toHaveCount(0);

      const multiStepResponse = await request.post(coreUrl('/mfa-flow'), {
        data: {
          title: 'e2e-free-api-multi-step',
          steps: [{ methods: ['totp'] }, { methods: ['email'] }],
        },
      });
      await expectLicenseError(multiStepResponse, 'steps', 'business_license_required');

      await saveMfaFlow(page);
      await page.getByTestId('add-mfa-flow').click();
      await expect(page.locator('#upgrade-business-modal')).toBeVisible();
      await expect(page).toHaveURL(routes.base + routes.mfa.list);

      const additionalFlowResponse = await request.post(coreUrl('/mfa-flow'), {
        data: { title: 'e2e-free-api-additional-flow', steps: [{ methods: ['totp'] }] },
      });
      await expectLicenseError(
        additionalFlowResponse,
        'flow',
        'additional_flow_business_license_required',
      );
    });

    test.describe('Multiple-assignment boundary', () => {
      test.skip(!seedLicenseKey, 'Requires a license to prepare multiple flows.');

      test('Free rejects multiple location flow assignments', async ({
        page,
        request,
      }) => {
        await setLicense(request, seedLicenseKey ?? '', seedLicenseTier);

        const defaultFlowId = await createMfaFlowViaApi(
          request,
          'e2e-free-assignment-default',
          [['totp']],
        );
        const secondFlowId = await createMfaFlowViaApi(
          request,
          'e2e-free-assignment-second',
          [['totp']],
        );
        const locationId = await apiCreateMfaLocation(request, defaultFlowId);

        await setLicense(request, null, null);
        await loginBasic(page, defaultUserAdmin);
        await page.goto(`${routes.base}${routes.locations}/${locationId}/edit`);
        await expect(page.getByTestId('location-mfa-add-override')).toBeDisabled();

        const updateResponse = await request.put(coreUrl(`/network/${locationId}`), {
          data: mfaLocationData([
            { flow_id: defaultFlowId, is_default: true, group_ids: [] },
            { flow_id: secondFlowId, is_default: false, group_ids: [] },
          ]),
        });
        expect(updateResponse.status()).toBe(200);
        expect(await getLocationAssignments(request, locationId)).toHaveLength(1);

        const createResponse = await request.post(coreUrl('/network'), {
          data: {
            ...mfaLocationData([
              { flow_id: defaultFlowId, is_default: true, group_ids: [] },
              { flow_id: secondFlowId, is_default: false, group_ids: [] },
            ]),
            name: 'e2e-free-rejected-assignment-location',
            address: '10.98.0.1/24',
            allowed_ips: '10.98.0.0/24',
            port: 51831,
          },
        });
        await expectLicenseError(
          createResponse,
          'mfa_flows',
          'multiple_mfa_flows_not_allowed',
        );
        await expectDefaultFlowAssignment(
          request,
          locationId,
          'e2e-free-assignment-default',
        );
      });
    });
  });

  test.describe('Business tier boundaries', () => {
    test.skip(!businessLicenseKey, 'Requires DEFGUARD_BUSINESS_LICENSE_KEY.');

    test('Business allows multi-step flows and their default assignment', async ({
      page,
      browser,
      request,
    }) => {
      await setLicense(request, businessLicenseKey ?? '', 'Business');
      await setupSMTP(browser);
      await loginBasic(page, defaultUserAdmin);

      const uiFlowTitle = 'e2e-business-ui-multi-step';
      await createMfaFlow(page, uiFlowTitle, [['totp'], ['email']]);
      const uiFlowId = await getFlowId(request, uiFlowTitle);
      const uiFlowResponse = await request.get(coreUrl(`/mfa-flow/${uiFlowId}`));
      expect((await uiFlowResponse.json()).steps.map((step) => step.methods)).toEqual([
        ['totp'],
        ['email'],
      ]);

      const apiFlowTitle = 'e2e-business-api-multi-step';
      const apiFlowId = await createMfaFlowViaApi(request, apiFlowTitle, [
        ['totp'],
        ['email'],
      ]);
      const locationId = await apiCreateMfaLocation(request, apiFlowId);
      await expectDefaultFlowAssignment(request, locationId, apiFlowTitle);
    });

    test('Business rejects group-scoped assignments in the UI and API', async ({
      page,
      browser,
      request,
    }) => {
      await setLicense(request, businessLicenseKey ?? '', 'Business');

      const defaultFlowId = await createMfaFlowViaApi(
        request,
        'e2e-business-assignment-default',
        [['totp']],
      );
      const overrideFlowId = await createMfaFlowViaApi(
        request,
        'e2e-business-assignment-override',
        [['totp']],
      );
      const locationId = await apiCreateMfaLocation(request, defaultFlowId);
      const groupName = 'e2e-business-mfa-group';
      await createGroup(browser, groupName);
      const groupId = await getGroupId(request, groupName);

      await loginBasic(page, defaultUserAdmin);
      await page.goto(`${routes.base}${routes.locations}/${locationId}/edit`);
      await expect(page.getByTestId('location-mfa-add-override')).toBeDisabled();

      const response = await request.put(coreUrl(`/network/${locationId}`), {
        data: mfaLocationData([
          { flow_id: defaultFlowId, is_default: true, group_ids: [] },
          { flow_id: overrideFlowId, is_default: false, group_ids: [groupId] },
        ]),
      });
      await expectLicenseError(response, 'mfa_flows', 'group_assignment_not_allowed');
      expect(await getLocationAssignments(request, locationId)).toHaveLength(1);
      await expectDefaultFlowAssignment(
        request,
        locationId,
        'e2e-business-assignment-default',
      );
    });
  });

  test.describe('Licensed flow capabilities', () => {
    test.skip(!seedLicenseKey, 'Requires a Business or Enterprise license.');

    test.beforeEach(async ({ request }) => {
      if (!enterpriseLicenseKey && businessLicenseKey) {
        await setLicense(request, businessLicenseKey, 'Business');
      }
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

    test('Enforce a TOTP then Email flow over Edge', async ({
      page,
      browser,
      request,
    }) => {
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
      if (secondResult.kind !== 'Completed')
        throw new Error('MFA flow did not complete.');
      expect(secondResult.presharedKey).toBeTruthy();
    });
  });
});
