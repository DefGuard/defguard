import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

import { OverviewLayoutType, OverviewStore } from '../../../../shared/types';

export const useOverviewStore = create<
  OverviewStore,
  [
    [
      'zustand/persist',
      Pick<OverviewStore, 'viewMode' | 'defaultViewMode' | 'statsFilter'>
    ]
  ]
>(
  persist(
    (set) => ({
      viewMode: OverviewLayoutType.GRID,
      defaultViewMode: OverviewLayoutType.GRID,
      statsFilter: 1,
      setState: (newValues) => set((state) => ({ ...state, ...newValues })),
    }),
    { name: 'overview-store', storage: createJSONStorage(() => sessionStorage) }
  )
);
