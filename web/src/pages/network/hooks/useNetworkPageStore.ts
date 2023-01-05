import { Subject } from 'rxjs';
import create from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

import { Network } from '../../../shared/types';

interface NetworkPageStore {
  saveSubject: Subject<void>;
  loading: boolean;
  network?: Network;
  setState: (data: Partial<NetworkPageStore>) => void;
}
export const useNetworkPageStore = create<NetworkPageStore>()(
  persist(
    (set) => ({
      saveSubject: new Subject(),
      loading: false,
      network: undefined,
      setState: (newState) => set(() => newState),
    }),
    {
      name: 'network-page',
      storage: createJSONStorage(() => sessionStorage),
      partialize: (state) => ({ network: state.network }),
    }
  )
);
