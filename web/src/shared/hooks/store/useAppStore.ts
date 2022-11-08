import create from 'zustand';
import { persist } from 'zustand/middleware';

import { UseAppStore } from '../../types';

export const useAppStore = create<
  UseAppStore,
  [
    [
      'zustand/persist',
      Pick<UseAppStore, 'backendVersion' | 'wizardCompleted' | 'settings'>
    ]
  ]
>(
  persist(
    (set) => ({
      backendVersion: undefined,
      wizardCompleted: undefined,
      settings: undefined,
      setAppStore: (data) => set((state) => ({ ...state, ...data })),
    }),
    {
      name: 'app-store',
      getStorage: () => sessionStorage,
    }
  )
);
