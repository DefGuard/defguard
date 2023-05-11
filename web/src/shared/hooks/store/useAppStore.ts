import { pick } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

import { AppStore } from '../../types';

export const useAppStore = create<AppStore>()(
  persist(
    (set) => ({
      backendVersion: undefined,
      settings: undefined,
      license: undefined,
      version: undefined,
      language: undefined,
      setAppStore: (data) => set((state) => ({ ...state, ...data })),
    }),
    {
      name: 'app-store',
      partialize: (store) => pick(store, ['settings', 'language']),
      storage: createJSONStorage(() => sessionStorage),
    }
  )
);
