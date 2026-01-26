import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import {
  type EditNetworkLocation,
  LocationMfaMode,
  LocationServiceMode,
} from '../../shared/api/types';
import { AddLocationPageStep, type AddLocationPageStepValue } from './types';

type StoreValues = {
  activeStep: AddLocationPageStepValue;
  locationType: 'regular' | 'service';
} & EditNetworkLocation;

type StoreMethods = {
  reset: () => void;
  start: (values?: Partial<StoreValues>) => void;
};

const defaults: StoreValues = {
  locationType: 'regular',
  activeStep: AddLocationPageStep.Start,
  // form values
  name: '',
  port: 50051,
  keepalive_interval: 25,
  mtu: 1420,
  fwmark: 0,
  peer_disconnect_threshold: 300,
  acl_default_allow: true,
  acl_enabled: false,
  address: '',
  allowed_groups: [],
  allowed_ips: '',
  dns: '',
  endpoint: '',
  location_mfa_mode: LocationMfaMode.Disabled,
  service_location_mode: LocationServiceMode.Disabled,
};

export const useAddLocationStore = create<StoreMethods & StoreValues>()(
  persist(
    (set) => ({
      ...defaults,
      reset: () => set(defaults),
      start: (initial) => {
        set({
          ...defaults,
          ...initial,
          activeStep: AddLocationPageStep.Start,
        });
      },
    }),
    {
      name: 'add-location-store',
      storage: createJSONStorage(() => sessionStorage),
      partialize: (state) => omit(state, ['reset', 'start']),
    },
  ),
);
