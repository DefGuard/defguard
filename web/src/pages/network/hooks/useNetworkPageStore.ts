import { Subject } from 'rxjs';
import { createWithEqualityFn } from 'zustand/traditional';

import type { Network } from '../../../shared/types';

type NetworkPageStore = {
  saveSubject: Subject<void>;
  loading: boolean;
  networks: Network[];
  selectedNetworkId?: number;
  setState: (data: Partial<NetworkPageStore>) => void;
  setNetworks: (data: Network[]) => void;
};

export const useNetworkPageStore = createWithEqualityFn<NetworkPageStore>()(
  (set, get) => ({
    saveSubject: new Subject(),
    loading: false,
    networks: [],
    selectedNetworkId: undefined,
    setState: (newState) => set(() => newState),
    setNetworks: (networks) => {
      const sortedNetworks = networks.sort((a, b) => a.name.localeCompare(b.name));
      if (get().selectedNetworkId === undefined) {
        set({ selectedNetworkId: sortedNetworks[0]?.id });
      }
      set({ networks: sortedNetworks });
    },
  }),
  Object.is,
);
