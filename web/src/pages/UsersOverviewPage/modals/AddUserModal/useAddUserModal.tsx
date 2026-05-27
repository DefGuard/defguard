import { create } from 'zustand';
import type { StartEnrollmentResponse, User } from '../../../../shared/api/types';

interface StoreValues {
  step: 'user' | 'groups' | 'enroll-choice' | 'enrollment';
  isOpen: boolean;
  enrollUser: boolean;
  groups: string[];
  user?: User;
  enrollResponse?: StartEnrollmentResponse;
}

const defaults: StoreValues = {
  isOpen: false,
  enrollUser: false,
  step: 'enroll-choice',
  groups: [],
  user: undefined,
  enrollResponse: undefined,
};

interface Store extends StoreValues {
  open: () => void;
  reset: () => void;
}

export const useAddUserModal = create<Store>((set) => ({
  ...defaults,
  reset: () => set(defaults),
  open: () =>
    set({
      isOpen: true,
    }),
}));
