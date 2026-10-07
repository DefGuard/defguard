import { APIRequestContext, APIResponse, expect } from '@playwright/test';
import { generateKeyPairSync } from 'crypto';

import { testsConfig } from '../../config';

export enum MfaMethod {
  TOTP = 0,
  EMAIL = 1,
}

// A legacy (pre-2.2) client can connect only when the flow holds this full method set.
const INTERNAL_METHODS = ['totp', 'email', 'biometric', 'mobileapprove'];

export type ClientProtocol = 'multi-step' | 'legacy';

export type ClientDevice = {
  locationId: number;
  pubkey: string;
  protocol: ClientProtocol;
  method: MfaMethod;
};

export type MfaAttempt = {
  token: string;
  stepAttemptId?: string;
};

const coreUrl = (path: string) => testsConfig.CORE_BASE_URL + path;
const proxyUrl = (path: string) => testsConfig.ENROLLMENT_URL + '/api/v1' + path;

export const waitForProxy = async (api: APIRequestContext): Promise<void> => {
  await expect
    .poll(async () => (await api.get(proxyUrl('/health-grpc'))).status(), {
      timeout: 30_000,
    })
    .toBe(200);
};

export const apiCreateMfaLocation = async (admin: APIRequestContext): Promise<number> => {
  const flow = await admin.post(coreUrl('/mfa-flow'), {
    data: { title: 'Internal MFA', steps: [{ methods: INTERNAL_METHODS }] },
  });
  expect(flow.status()).toBe(201);
  const { id: flowId } = await flow.json();

  const location = await admin.post(coreUrl('/network'), {
    data: {
      name: 'mfa-location',
      address: '10.99.0.1/24',
      port: 51830,
      endpoint: '192.168.4.14',
      allowed_ips: '10.99.0.0/24',
      dns: null,
      mtu: 1420,
      fwmark: 0,
      allowed_groups: [],
      allow_all_groups: true,
      keepalive_interval: 25,
      peer_disconnect_threshold: 300,
      acl_enabled: false,
      acl_default_allow: false,
      mfa_enabled: true,
      service_location_mode: 'disabled',
      posture_checks: [],
      mfa_flows: [{ flow_id: flowId, is_default: true, group_ids: [] }],
    },
  });
  expect(location.status()).toBe(201);
  return (await location.json()).id;
};

export const apiCreateDevice = async (
  admin: APIRequestContext,
  username: string,
  name: string,
): Promise<string> => {
  const pubkey = generateKeyPairSync('x25519')
    .publicKey.export({ format: 'der', type: 'spki' })
    .subarray(-32)
    .toString('base64');
  const response = await admin.post(coreUrl(`/device/${username}`), {
    data: { name, wireguard_pubkey: pubkey },
  });
  expect(response.status()).toBe(201);
  return pubkey;
};

export const clientMfaStart = (
  api: APIRequestContext,
  device: ClientDevice,
): Promise<APIResponse> =>
  device.protocol === 'legacy'
    ? api.post(proxyUrl('/client-mfa/start'), {
        data: {
          location_id: device.locationId,
          pubkey: device.pubkey,
          method: device.method,
        },
      })
    : api.post(proxyUrl('/mfa-flow/start'), {
        data: {
          location_id: device.locationId,
          pubkey: device.pubkey,
          selected_methods: [device.method],
        },
      });

export const clientMfaStepStart = (
  api: APIRequestContext,
  token: string,
  method: MfaMethod,
): Promise<APIResponse> =>
  api.post(proxyUrl('/mfa-flow/step-start'), { data: { token, method } });

export const clientMfaConnect = async (
  api: APIRequestContext,
  device: ClientDevice,
): Promise<MfaAttempt> => {
  const start = await clientMfaStart(api, device);
  expect(start.status()).toBe(200);
  const body = await start.json();
  if (device.protocol === 'legacy') {
    expect(body.token).toBeTruthy();
    return { token: body.token };
  }
  const accepted = body.outcome?.Accepted;
  expect(accepted?.token).toBeTruthy();
  return { token: accepted.token, stepAttemptId: accepted.first_step.step_attempt_id };
};

export const clientMfaFinish = (
  api: APIRequestContext,
  attempt: MfaAttempt,
  code: string,
): Promise<APIResponse> =>
  attempt.stepAttemptId === undefined
    ? api.post(proxyUrl('/client-mfa/finish'), { data: { token: attempt.token, code } })
    : api.post(proxyUrl('/mfa-flow/step-finish'), {
        data: {
          token: attempt.token,
          step_attempt_id: attempt.stepAttemptId,
          submission: { Code: { code } },
        },
      });

// A multi-step client gets the key inside the completed step result.
export const presharedKey = async (
  response: APIResponse,
): Promise<string | undefined> => {
  const body = await response.json();
  return body.preshared_key ?? body.result?.outcome?.Completed?.preshared_key;
};
