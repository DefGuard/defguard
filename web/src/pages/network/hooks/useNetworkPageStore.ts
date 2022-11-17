import { Subject } from 'rxjs';
import create from 'zustand';

import { Network } from '../../../shared/types';

interface NetworkPageStore {
  saveSubject: Subject<void>;
  formValid: boolean;
  loading: boolean;
  network?: Network;
  setState: (data: Partial<NetworkPageStore>) => void;
}
export const useNetworkPageStore = create<NetworkPageStore>((set) => ({
  saveSubject: new Subject(),
  formValid: false,
  loading: false,
  network: undefined,
  setState: (newState) => set(() => newState),
}));
