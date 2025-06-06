import { createWithEqualityFn } from 'zustand/traditional';

import {
  ActivityLogStream,
  ActivityLogStreamLogstashHttp,
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
            'Opened Logstash Http CE modal with wrong activity log stream type config',
          );
        }
        const initData: ModifyData = {
          config: vals.config as ActivityLogStreamLogstashHttp,
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
  config: ActivityLogStreamLogstashHttp;
};

type StoreValues = {
  visible: boolean;
  initStreamData?: ModifyData;
};

type StoreMethods = {
  open: (activityLogStream?: ActivityLogStream) => void;
  close: () => void;
  reset: () => void;
};
