/* eslint-disable @typescript-eslint/no-unused-vars */
import { pick } from 'lodash-es';
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

import { NavigationStore } from '../../types';

export const useNavigationStore = create<NavigationStore>()(
  persist(
    (set, get) => ({
      isNavigationOpen: false,
      user: undefined,
      webhook: undefined,
      openidclient: undefined,
      enableWizard: undefined,
      setNavigationUser: (user) => set(() => ({ user: user })),
      setNavigationWebhook: (webhook) => set(() => ({ webhook: webhook })),
      setNavigationOpenidClient: (openidclient) =>
        set(() => ({ openidclient: openidclient })),
      setNavigationOpen: (v) => set(() => ({ isNavigationOpen: v })),
      setState: (newState) => set({ ...get(), ...newState }),
    }),
    {
      version: 1.2,
      name: 'navigation-store',
      partialize: (state) => pick(state, ['isNavigationOpen']),
    }
  )
);
