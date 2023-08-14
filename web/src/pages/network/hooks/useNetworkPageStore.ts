import { Subject } from 'rxjs';
import { createWithEqualityFn } from 'zustand/traditional';

import { Network } from '../../../shared/types';

type NetworkPageStore = {
  saveSubject: Subject<void>;
  loading: boolean;
  networks: Network[];
  selectedNetworkId: number;
  setState: (data: Partial<NetworkPageStore>) => void;
};

export const useNetworkPageStore = createWithEqualityFn<NetworkPageStore>()(
  (set) => ({
    saveSubject: new Subject(),
    loading: false,
    networks: [],
    selectedNetworkId: 1,
    setState: (newState) => set(() => newState),
  }),
  Object.is,
);
