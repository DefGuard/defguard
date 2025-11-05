import { create } from 'zustand';
import type { User } from '../../../../shared/api/types';

interface StoreValues {
  step: 'user' | 'groups';
  isOpen: boolean;
  reservedEmails: string[];
  reservedUsernames: string[];
  user?: User;
}

const defaults: StoreValues = {
  isOpen: false,
  step: 'user',
  reservedEmails: [],
  reservedUsernames: [],
  user: undefined,
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
