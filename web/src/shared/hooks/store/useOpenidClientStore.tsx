import create from 'zustand';

import { OpenidClientStore } from '../../types';

export const useOpenidClientStore = create<OpenidClientStore>((set) => ({
  editMode: false,
  setEditMode: (v) => set(() => ({ editMode: v })),
}));
