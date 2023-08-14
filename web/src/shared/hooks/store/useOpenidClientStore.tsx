import { createWithEqualityFn } from 'zustand/traditional';

import { OpenidClientStore } from '../../types';

export const useOpenidClientStore = createWithEqualityFn<OpenidClientStore>(
  (set) => ({
    editMode: false,
    setEditMode: (v) => set(() => ({ editMode: v })),
  }),
  Object.is,
);
