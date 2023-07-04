import { Subject } from 'rxjs';
import { create } from 'zustand';

import { UserProfile } from '../../types';

const defaultValues: StoreValues = {
  editMode: false,
  loading: false,
  isMe: false,
  submitSubject: new Subject<void>(),
  userProfile: undefined,
};

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export const useUserProfileStore = create<Store>((set) => ({
  ...defaultValues,
  setState: (newState) => set((oldState) => ({ ...oldState, ...newState })),
}));

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
};
