import { create } from 'zustand';
import type { StartEnrollmentResponse, User } from '../../../../shared/api/types';

interface StoreValues {
  step: 'user' | 'groups' | 'enroll-choice' | 'enrollment';
  isOpen: boolean;
  enrollUser: boolean;
  reservedEmails: string[];
  reservedUsernames: string[];
  groups: string[];
  user?: User;
  enrollResponse?: StartEnrollmentResponse;
}

const defaults: StoreValues = {
  isOpen: false,
  enrollUser: false,
  step: 'enroll-choice',
  reservedEmails: [],
  reservedUsernames: [],
  groups: [],
  user: undefined,
  enrollResponse: undefined,
};

interface Store extends StoreValues {
  open: (data: Pick<StoreValues, 'reservedEmails' | 'reservedUsernames'>) => void;
  reset: () => void;
}

export const useAddUserModal = create<Store>((set) => ({
  ...defaults,
  reset: () => set(defaults),
  open: (data) =>
    set({
      isOpen: true,
      ...data,
    }),
}));
