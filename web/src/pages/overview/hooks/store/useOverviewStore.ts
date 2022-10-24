import create from 'zustand';

import { OverviewLayoutType, OverviewStore } from '../../../../shared/types';

export const useOverviewStore = create<OverviewStore>((set) => ({
  viewMode: OverviewLayoutType.GRID,
  statsFilter: 1,
  setState: (newValues) => set((state) => ({ ...state, ...newValues })),
}));
