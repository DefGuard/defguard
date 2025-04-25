import { Axios } from 'axios';
import { createWithEqualityFn } from 'zustand/traditional';

import { ApiHook } from '../../types';
import { buildApi } from './api';

const defaults: StoreValues = {
  client: undefined,
  endpoints: undefined,
};

export const useApiStore = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    init: (client) => {
      const endpoints = buildApi(client);
      set({
        client,
        endpoints,
      });
    },
  }),
  Object.is,
);

type Store = StoreMethods & StoreValues;

type StoreValues = {
  client?: Axios;
  endpoints?: ApiHook;
};

type StoreMethods = {
  init: (client: Axios) => void;
};
