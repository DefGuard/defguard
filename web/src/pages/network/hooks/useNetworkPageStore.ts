import { Subject } from 'rxjs';

import { Network } from '../../../shared/types';
import { createWithEqualityFn } from 'zustand/traditional';

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
