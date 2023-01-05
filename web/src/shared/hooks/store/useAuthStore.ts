import create from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

import { isUserAdmin } from '../../helpers/isUserAdmin';
import { AuthStore } from '../../types';

const storeDefaultValues = {
  user: undefined,
  isAdmin: undefined,
  authLocation: undefined,
};

export const useAuthStore = create<
  AuthStore,
  [['zustand/persist', Pick<AuthStore, 'user' | 'isAdmin' | 'authLocation'>]]
>(
  persist(
    (set) => ({
      ...storeDefaultValues,
      setState: (newState) => set((state) => ({ ...state, ...newState })),
      logIn: (user) =>
        set((state) => ({
          ...state,
          user: user,
          isAdmin: isUserAdmin(user),
        })),
      logOut: () => set(() => storeDefaultValues),
    }),
    {
      name: 'auth-store',
      storage: createJSONStorage(() => sessionStorage),
    }
  )
);
