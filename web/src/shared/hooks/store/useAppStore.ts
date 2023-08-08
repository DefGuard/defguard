import { pick } from 'lodash-es';
import { createJSONStorage, persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import { AppStore } from '../../types';

export const useAppStore = createWithEqualityFn<AppStore>()(
  persist(
    (set) => ({
      settings: undefined,
      license: undefined,
      language: undefined,
      appInfo: undefined,
      setAppStore: (data) => set((state) => ({ ...state, ...data })),
    }),
    {
      name: 'app-store',
      partialize: (store) => pick(store, ['settings', 'language']),
      storage: createJSONStorage(() => sessionStorage),
    },
  ),
  Object.is,
);
