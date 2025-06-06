import { createWithEqualityFn } from 'zustand/traditional';

import { isPresent } from '../../../../../../shared/defguard-ui/utils/isPresent';
import { ActivityStreamVectorHttp } from '../../../../../../shared/types';

type ModifyData = {
  id: number;
  name: string;
  config: ActivityStreamVectorHttp;
};

const defaults: StoreValues = {
  visible: false,
  edit: false,
  initStreamData: undefined,
};

export const useVectorHttpStreamCEModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (initData) => {
      if (isPresent(initData)) {
        set({ visible: true, edit: true, initStreamData: initData });
      }
      set({ visible: true, edit: true });
    },
    close: () => set({ visible: false }),
    reset: () => set(defaults),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  edit: boolean;
  initStreamData?: ModifyData;
};

type StoreMethods = {
  open: (values?: ModifyData) => void;
  close: () => void;
  reset: () => void;
};
