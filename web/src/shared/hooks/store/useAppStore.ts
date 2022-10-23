import create from 'zustand';

import { UseAppStore } from '../../types';

export const useAppStore = create<UseAppStore>((set) => ({
  backendVersion: undefined,
  wizardCompleted: undefined,
  setAppStore: (data) => set((state) => ({ ...state, ...data })),
}));
