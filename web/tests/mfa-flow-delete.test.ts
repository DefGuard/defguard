import { AxiosError, type AxiosResponse } from 'axios';
import { describe, expect, it, vi } from 'vitest';
import {
  getDeleteMfaFlowModalData,
  getMfaFlowDeleteBlockingLocations,
  getMfaFlowDeleteConflictLocations,
} from '../src/pages/MfaPage/mfaFlows';
import api from '../src/shared/api/api';
import type { MfaFlowLocation } from '../src/shared/api/types';

const location = (
  name: string,
  is_default: boolean,
  mfa_enabled: boolean,
): MfaFlowLocation => ({ id: name.length, name, is_default, mfa_enabled });

const responseError = (status: number, data: unknown) =>
  new AxiosError('request failed', String(status), undefined, undefined, {
    status,
    data,
  } as AxiosResponse);

describe('MFA flow delete confirmation', () => {
  it('invalidates the location assignment queries so deleted flows leave no ghost row', async () => {
    const deleteFlow = vi
      .spyOn(api.mfaFlow, 'delete')
      .mockResolvedValue(undefined as never);

    const modalData = getDeleteMfaFlowModalData({ id: 7, title: 'Contractors' });

    expect(modalData.invalidateKeys).toEqual([['mfa-flow'], ['location']]);

    await modalData.actionPromise();

    expect(deleteFlow).toHaveBeenCalledWith(7);
  });
});

describe('MFA flow delete block', () => {
  it('blocks only on locations where the flow is the default and MFA is enabled', () => {
    const locations = [
      location('Berlin', true, true),
      location('Warsaw', true, false),
      location('Prague', false, true),
    ];

    expect(getMfaFlowDeleteBlockingLocations({ locations })).toEqual(['Berlin']);
    expect(getMfaFlowDeleteBlockingLocations({ locations: [] })).toEqual([]);
  });

  it('reads the location names from a 409 refusal and ignores other errors', () => {
    const conflict = responseError(409, {
      error: 'conflict',
      fields: [{ field: 'id', code: 'flow_is_default', locations: ['Berlin'] }],
    });

    expect(getMfaFlowDeleteConflictLocations(conflict)).toEqual(['Berlin']);
    expect(getMfaFlowDeleteConflictLocations(responseError(500, {}))).toBeNull();
    expect(getMfaFlowDeleteConflictLocations(new Error('network'))).toBeNull();
  });
});
