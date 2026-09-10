import { describe, expect, it, vi } from 'vitest';
import { getDeleteMfaFlowModalData } from '../src/pages/MfaPage/mfaFlows';
import api from '../src/shared/api/api';

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
