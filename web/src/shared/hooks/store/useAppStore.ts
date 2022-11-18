import create from 'zustand';
import { persist } from 'zustand/middleware';

import { UseAppStore } from '../../types';

export const useAppStore = create<
  UseAppStore,
  [['zustand/persist', Pick<UseAppStore, 'settings'>]]
>(
  persist(
    (set) => ({
      backendVersion: undefined,
      settings: undefined,
      setAppStore: (data) => set((state) => ({ ...state, ...data })),
    }),
    {
      name: 'app-store',
      getStorage: () => sessionStorage,
    }
  )
);
