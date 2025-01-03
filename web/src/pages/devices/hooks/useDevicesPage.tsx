import { useState } from 'react';
import { createContainer } from 'react-tracked';

import { StandaloneDevice } from '../../../shared/types';

export type DevicesPageContext = {
  devices: StandaloneDevice[];
  reservedUserDeviceNames: string[];
  reservedNetworkDeviceNames: string[];
  search: string;
};

const initialState: DevicesPageContext = {
  devices: [],
  reservedUserDeviceNames: [],
  reservedNetworkDeviceNames: [],
  search: '',
};

const useValue = () => useState(initialState);

export const { Provider: DevicesPageProvider, useTracked: useDevicesPage } =
  createContainer(useValue);
