import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type { LoginMfaResponse, User } from '../api/types';

type Store = Values & Methods;

type Values = {
  isAdmin: boolean;
  isAuthenticated: boolean;
  user?: User;
  mfaLogin?: LoginMfaResponse;
};

type Methods = {
  setUser: (values?: User) => void;
  reset: () => void;
};

const defaults: Values = {
  isAdmin: false,
  isAuthenticated: false,
  user: undefined,
  mfaLogin: undefined,
};

export const useAuth = create<Store>()(
  persist(
    (set) => ({
      ...defaults,
      setUser: (user) => {
        if (user) {
          set({
            isAdmin: user.is_admin,
            isAuthenticated: true,
            user: user,
            mfaLogin: undefined,
          });
        } else {
          set(defaults);
        }
      },
      reset: () => set(defaults),
    }),
    {
      name: 'auth-store',
      version: 1,
      storage: createJSONStorage(() => sessionStorage),
    },
  ),
);
