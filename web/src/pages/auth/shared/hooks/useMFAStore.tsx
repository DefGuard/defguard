import create from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

import { MFALoginResponse, UserMFAMethod } from '../../../../shared/types';

interface MFAStore extends MFALoginResponse {
  setState: (data: Partial<MFAStore>) => void;
  resetState: () => void;
}

const defaultState: MFALoginResponse = {
  mfa_method: UserMFAMethod.NONE,
  totp_available: false,
  web3_available: false,
  webauthn_available: false,
};

export const useMFAStore = create<MFAStore, [['zustand/persist', MFAStore]]>(
  persist(
    (set) => ({
      ...defaultState,
      setState: (newValues) => set((state) => ({ ...state, ...newValues })),
      resetState: () => set(() => defaultState),
    }),
    {
      name: 'mfa-storage',
      storage: createJSONStorage(() => sessionStorage),
    }
  )
);
