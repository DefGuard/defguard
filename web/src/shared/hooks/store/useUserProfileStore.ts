import create from 'zustand';

import { UserProfileStore } from '../../types';

export const useUserProfileStore = create<UserProfileStore>((set) => ({
  editMode: false,
  setEditMode: (v) => set(() => ({ editMode: v })),
}));
