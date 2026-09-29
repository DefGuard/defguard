import { describe, expect, it } from 'vitest';
import { m } from '../src/paraglide/messages';
import {
  type MfaFlowListItemResponse,
  MfaFlowMethod,
  type MfaFlowMethodValue,
} from '../src/shared/api/types';
import { openIdProviderDeleteBody } from '../src/shared/utils/mfaFlowSteps';

type Flow = Pick<MfaFlowListItemResponse, 'title' | 'steps'>;

const flow = (title: string, methods: MfaFlowMethodValue[][]): Flow => ({
  title,
  steps: methods.map((stepMethods, index) => ({
    id: index + 1,
    position: index,
    methods: stepMethods,
  })),
});

describe('openIdProviderDeleteBody', () => {
  it('names only the flows that reference the external provider, including later steps', () => {
    const body = openIdProviderDeleteBody([
      flow('Employees', [[MfaFlowMethod.Totp], [MfaFlowMethod.Email]]),
      flow('Contractors', [[MfaFlowMethod.Totp], [MfaFlowMethod.OpenId]]),
      flow('Admins', [[MfaFlowMethod.OpenId, MfaFlowMethod.Fido2]]),
    ]);

    expect(body).toBe(
      m.settings_openid_provider_delete_confirm_body_mfa({
        flows: 'Contractors, Admins',
      }),
    );
  });

  it('keeps the plain body when no flow uses the external provider', () => {
    const body = openIdProviderDeleteBody([flow('Employees', [[MfaFlowMethod.Totp]])]);

    expect(body).toBe(m.settings_openid_provider_delete_confirm_body());
  });
});
