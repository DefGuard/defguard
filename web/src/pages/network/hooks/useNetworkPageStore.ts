import { Subject } from 'rxjs';
import { createWithEqualityFn } from 'zustand/traditional';

import { Network } from '../../../shared/types';

type NetworkPageStore = {
  saveSubject: Subject<void>;
  loading: boolean;
  networks: Network[];
  selectedNetworkId: number;
  setState: (data: Partial<NetworkPageStore>) => void;
  setNetworks: (data: Network[]) => void;
};

export const useNetworkPageStore = createWithEqualityFn<NetworkPageStore>()(
  (set, get) => ({
    saveSubject: new Subject(),
    loading: false,
    networks: [],
    selectedNetworkId: 1,
    setState: (newState) => set(() => newState),
    setNetworks: (networks) => {
      if (get().selectedNetworkId === undefined) {
        set({ selectedNetworkId: networks[0]?.id });
      }
      set({ networks });
    },
  }),
  Object.is,
);
