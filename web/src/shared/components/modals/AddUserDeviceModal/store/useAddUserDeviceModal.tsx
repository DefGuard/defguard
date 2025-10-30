import { create } from 'zustand';
import type { User, UserDevice } from '../../../../api/types';
import { AddUserDeviceModalStep, type AddUserDeviceModalStepValue } from '../types';

interface StoreValues {
  isOpen: boolean;
  step: AddUserDeviceModalStepValue;
  enrollment?: {
    token: string;
    url: string;
  };
  user?: User;
  devices?: UserDevice[];
  networks?: Array<{
    id: number;
    name: string;
  }>;
  manualConfig?: {
    publicKey: string;
    privateKey?: string;
  };
}

type OpenValues = {
  user: User;
  devices: UserDevice[];
};

interface Store extends StoreValues {
  reset: () => void;
  open: (openValues: OpenValues) => void;
  close: () => void;
}

const defaults: StoreValues = {
  isOpen: false,
  step: AddUserDeviceModalStep.StartChoice,
  devices: undefined,
  user: undefined,
  networks: undefined,
  manualConfig: undefined,
  enrollment: undefined,
};

export const useAddUserDeviceModal = create<Store>((set) => ({
  ...defaults,
  reset: () => set(defaults),
  open: (values) => set({ ...values, isOpen: true }),
  close: () => set({ isOpen: false }),
}));
