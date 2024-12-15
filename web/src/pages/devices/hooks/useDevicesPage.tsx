import { useState } from 'react';
import { createContainer } from 'react-tracked';

type MockedLocation = {
  id: number;
  name: string;
};

export type MockDevice = {
  id: number;
  name: string;
  assignedIp: string;
  description: string;
  addedBy: string;
  // ISO date
  addedDate: string;
  location: MockedLocation[];
};

export type DevicesPageContext = {
  devices: MockDevice[];
  search: string;
};

const initialState: DevicesPageContext = {
  devices: [],
  search: '',
};

const useValue = () => useState(initialState);

export const { Provider: DevicesPageProvider, useTracked: useDevicesPage } =
  createContainer(useValue);
