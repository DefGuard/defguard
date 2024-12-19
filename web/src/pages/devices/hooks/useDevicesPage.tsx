import { useState } from 'react';
import { createContainer } from 'react-tracked';

import { StandaloneDevice } from '../../../shared/types';

export type DevicesPageContext = {
  devices: StandaloneDevice[];
  search: string;
};

const initialState: DevicesPageContext = {
  devices: [],
  search: '',
};

const useValue = () => useState(initialState);

export const { Provider: DevicesPageProvider, useTracked: useDevicesPage } =
  createContainer(useValue);
