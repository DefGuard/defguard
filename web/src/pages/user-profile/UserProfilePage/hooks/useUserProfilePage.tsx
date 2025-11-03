import { createContext, useContext } from 'react';
import { createStore, useStore } from 'zustand';
import type { AuthKey, UserProfile } from '../../../../shared/api/types';

interface Extras {
  authKeys: AuthKey[];
}

interface ProfileProps extends Extras {
  profile: UserProfile;
}

interface ProfileState extends UserProfile, Extras {
  reset: () => void;
}

type UserProfileStore = ReturnType<typeof createUserProfileStore>;

export const createUserProfileStore = (initialProps: ProfileProps) => {
  return createStore<ProfileState>()((set) => ({
    authKeys: initialProps.authKeys,
    devices: initialProps.profile.devices,
    security_keys: initialProps.profile.security_keys,
    user: initialProps.profile.user,
    reset: () => set(initialProps.profile),
  }));
};

export const UserProfileContext = createContext<UserProfileStore | null>(null);

export function useUserProfile<T>(selector: (state: ProfileState) => T): T {
  const store = useContext(UserProfileContext);

  if (!store) throw new Error('Missing userProfile context');

  return useStore(store, selector);
}
