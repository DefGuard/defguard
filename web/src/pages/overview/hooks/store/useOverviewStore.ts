import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

import { OverviewLayoutType, OverviewStore } from '../../../../shared/types';

export const useOverviewStore = create<
  OverviewStore,
  [['zustand/persist', Omit<OverviewStore, 'setState' | 'networks'>]]
>(
  persist(
    (set) => ({
      selectedNetworkId: 1,
      networks: [],
      viewMode: OverviewLayoutType.GRID,
      defaultViewMode: OverviewLayoutType.GRID,
      statsFilter: 1,
      setState: (newValues) => set((state) => ({ ...state, ...newValues })),
    }),
    {
      version: 0.21,
      name: 'overview-store',
      storage: createJSONStorage(() => sessionStorage),
      partialize: (store) => omit(store, ['setState', 'networks']),
    }
  )
);
