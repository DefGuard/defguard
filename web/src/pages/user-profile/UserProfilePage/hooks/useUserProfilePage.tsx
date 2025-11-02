import { createContext, useContext } from 'react';
import { createStore, useStore } from 'zustand';
import type { UserProfile } from '../../../../shared/api/types';

interface ProfileProps {
  profile: UserProfile;
}

interface ProfileState extends UserProfile {
  reset: () => void;
}

type UserProfileStore = ReturnType<typeof createUserProfileStore>;

export const createUserProfileStore = (initialProps: ProfileProps) => {
  return createStore<ProfileState>()((set) => ({
    ...initialProps.profile,
    reset: () => set(initialProps.profile),
  }));
};

export const UserProfileContext = createContext<UserProfileStore | null>(null);

export function useUserProfile<T>(selector: (state: ProfileState) => T): T {
  const store = useContext(UserProfileContext);

  if (!store) throw new Error('Missing userProfile context');

  return useStore(store, selector);
}
