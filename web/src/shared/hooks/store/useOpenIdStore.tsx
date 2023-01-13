import create from 'zustand';

import { UseOpenIDStore } from '../../types';

export const useOpenIDStore = create<UseOpenIDStore>((set) => ({
	openIDRedirect: false,
  setOpenIDStore: (data) => set((state) => ({ ...state, ...data })),
}));
