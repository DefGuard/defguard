import { Axios } from 'axios';
import { createWithEqualityFn } from 'zustand/traditional';

import { Api } from '../../types';
import { buildApi } from './api';
import apiEndpoints from './api-client';
import axiosClient from './axios-client';

const defaults: StoreValues = {
  client: axiosClient,
  endpoints: apiEndpoints,
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
  endpoints?: Api;
};

type StoreMethods = {
  init: (client: Axios) => void;
};
