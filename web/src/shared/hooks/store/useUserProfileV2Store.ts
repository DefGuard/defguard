import { Subject } from 'rxjs';
import create from 'zustand';

import { User } from '../../types';

export interface UserProfileV2Store {
  editMode: boolean;
  isMe: boolean;
  user?: User;
  submitSubject: Subject<void>;
  loading: boolean;
  setState: (state: Partial<UserProfileV2Store>) => void;
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export const useUserProfileV2Store = create<UserProfileV2Store>((set, get) => ({
  editMode: false,
  loading: false,
  isMe: false,
  submitSubject: new Subject<void>(),
  setState: (newState) => set((oldState) => ({ ...oldState, ...newState })),
}));
