import create from 'zustand';
import { persist } from 'zustand/middleware';

import { UseAppStore } from '../../types';

export const useAppStore = create<
  UseAppStore,
  [
    [
      'zustand/persist',
      Pick<UseAppStore, 'backendVersion' | 'wizardCompleted' | 'settings' | 'settingsEditMode'>
    ]
  ]
>(
  persist(
    (set) => ({
      backendVersion: undefined,
      wizardCompleted: undefined,
			settingsEditMode: false,
      settings: undefined,
      setAppStore: (data) => set((state) => ({ ...state, ...data })),
    }),
    {
      name: 'app-store',
      getStorage: () => sessionStorage,
    }
  )
);
