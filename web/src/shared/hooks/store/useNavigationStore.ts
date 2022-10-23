/* eslint-disable @typescript-eslint/no-unused-vars */
import { pick } from 'lodash-es';
import create from 'zustand';
import { persist } from 'zustand/middleware';

import { NavigationStore } from '../../types';

export const useNavigationStore = create<
  NavigationStore,
  [['zustand/persist', Pick<NavigationStore, 'isNavigationOpen'>]]
>(
  persist(
    (set, _) => ({
      isNavigationOpen: false,
      user: undefined,
      webhook: undefined,
      openidclient: undefined,
      setNavigationUser: (user) => set(() => ({ user: user })),
      setNavigationWebhook: (webhook) => set(() => ({ webhook: webhook })),
      setNavigationOpenidClient: (openidclient) =>
        set(() => ({ openidclient: openidclient })),
      setNavigationOpen: (v) => set(() => ({ isNavigationOpen: v })),
    }),
    {
      version: 1.1,
      name: 'navigation-store',
      partialize: (state) => pick(state, ['isNavigationOpen']),
    }
  )
);
