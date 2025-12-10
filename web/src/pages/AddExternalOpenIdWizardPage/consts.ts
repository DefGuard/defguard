import api from '../../shared/api/api';
import type { TestDirectorySyncResponse } from '../../shared/api/types';
import { useAddExternalOpenIdStore } from './useAddExternalOpenIdStore';

export const validateExternalProviderWizard = async (): Promise<
  TestDirectorySyncResponse | boolean
> => {
  const state = useAddExternalOpenIdStore.getState().providerState;
  try {
    await api.openIdProvider.addOpenIdProvider({
      ...state,
    });
    if (state.enableDirectorySync) {
      const { data: result } = await api.openIdProvider.testDirectorySync();
      return result;
    }
  } catch (_) {
    return false;
  }
  return true;
};
