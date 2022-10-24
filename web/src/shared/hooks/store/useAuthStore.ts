import create from 'zustand';

import { isUserAdmin } from '../../helpers/isUserAdmin';
import { AuthStore } from '../../types';

const storeDefaultValues = {
  user: undefined,
  isAdmin: undefined,
};

export const useAuthStore = create<AuthStore>((set) => ({
  ...storeDefaultValues,
  setState: (newState) => set((state) => ({ ...state, ...newState })),
  logIn: (user) =>
    set((state) => ({
      ...state,
      user: user,
      isAdmin: isUserAdmin(user),
    })),
  logOut: () => set(() => storeDefaultValues),
}));
