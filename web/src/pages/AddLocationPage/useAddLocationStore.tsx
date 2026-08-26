import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import { type EditNetworkLocation, LocationServiceMode } from '../../shared/api/types';
import { AddLocationPageStep, type AddLocationPageStepValue } from './types';

type StoreValues = {
  isWelcome: boolean;
  activeStep: AddLocationPageStepValue;
  locationType: 'regular' | 'service';
  posture_checks: number[];
} & EditNetworkLocation;

type StoreMethods = {
  reset: () => void;
  start: (values?: Partial<StoreValues>) => void;
};

const defaults: StoreValues = {
  isWelcome: true,
  locationType: 'regular',
  activeStep: AddLocationPageStep.Start,
  // form values
  name: '',
  port: 50051,
  keepalive_interval: 25,
  mtu: 1420,
  fwmark: 0,
  allow_all_groups: true,
  peer_disconnect_threshold: 300,
  acl_default_allow: true,
  acl_enabled: false,
  allowed_ips_from_acl: false,
  address: '',
  allowed_groups: [],
  allowed_ips: '',
  dns: '',
  endpoint: '',
  mfa_enabled: false,
  service_location_mode: LocationServiceMode.Disabled,
  posture_checks: [],
  mfa_flows: [],
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
