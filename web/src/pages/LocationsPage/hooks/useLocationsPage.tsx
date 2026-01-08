import { create } from 'zustand';

type Store = {
  // when entering the page modal with gatewaySetup will be showed, then this value will reset, this value needs to be set on ID of a network modal will use.
  networkGatewayStartup?: number;
};

const defaults: Store = {
  networkGatewayStartup: undefined,
};

export const useLocationsPageStore = create<Store>(() => ({
  ...defaults,
}));
