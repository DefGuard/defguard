import { Subject } from 'rxjs';

import { createWithEqualityFn } from 'zustand/traditional';
import { UserProfile } from '../../types';

const defaultValues: StoreValues = {
  editMode: false,
  loading: false,
  isMe: false,
  submitSubject: new Subject<void>(),
  userProfile: undefined,
};

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export const useUserProfileStore = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    setState: (newState) => set((oldState) => ({ ...oldState, ...newState })),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  editMode: boolean;
  isMe: boolean;
  userProfile?: UserProfile;
  submitSubject: Subject<void>;
  loading: boolean;
};

type StoreMethods = {
  setState: (state: Partial<StoreValues>) => void;
  reset: () => void;
};
