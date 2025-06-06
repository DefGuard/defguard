import { createWithEqualityFn } from 'zustand/traditional';

import {
  ActivityStream,
  ActivityStreamLogstashHttp,
} from '../../../../../../shared/types';

const defaults: StoreValues = {
  visible: false,
};

export const useLogstashHttpStreamCEModalStore = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (vals) => {
      if (vals) {
        if (vals?.stream_type !== 'logstash_http') {
          throw Error(
            'Opened Logstash Http CE modal with wrong audit stream type config',
          );
        }
        const initData: ModifyData = {
          config: vals.config as ActivityStreamLogstashHttp,
          id: vals.id,
          name: vals.name,
        };
        set({ ...vals, visible: true, initStreamData: initData });
      }
      set({ visible: true });
    },
    close: () => set({ visible: false }),
    reset: () => set(defaults),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type ModifyData = {
  id: number;
  name: string;
  config: ActivityStreamLogstashHttp;
};

type StoreValues = {
  visible: boolean;
  initStreamData?: ModifyData;
};

type StoreMethods = {
  open: (activityStream?: ActivityStream) => void;
  close: () => void;
  reset: () => void;
};
