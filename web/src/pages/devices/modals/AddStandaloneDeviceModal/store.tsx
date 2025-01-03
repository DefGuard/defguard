import { isObject } from 'lodash-es';
import { Subject } from 'rxjs';
import { createWithEqualityFn } from 'zustand/traditional';

import { SelectOption } from '../../../../shared/defguard-ui/components/Layout/Select/types';
import {
  CreateStandaloneDeviceResponse,
  GetAvailableLocationIpResponse,
  Network,
  StartEnrollmentResponse,
} from '../../../../shared/types';
import {
  AddStandaloneDeviceModalChoice,
  AddStandaloneDeviceModalStep,
  WGConfigGenChoice,
} from './types';

const defaultValues: StoreValues = {
  visible: false,
  currentStep: AddStandaloneDeviceModalStep.METHOD_CHOICE,
  choice: AddStandaloneDeviceModalChoice.CLI,
  networks: undefined,
  networkOptions: [],
  genChoice: WGConfigGenChoice.AUTO,
  submitSubject: new Subject<void>(),
  initLocationIpResponse: undefined,
  genKeys: undefined,
  manualResponse: undefined,
  enrollResponse: undefined,
};

export const useAddStandaloneDeviceModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    setStore: (v) => set(v),
    reset: () => set(defaultValues),
    close: () => set({ visible: false }),
    open: () => set({ ...defaultValues, visible: true }),
    changeStep: (step) => set({ currentStep: step }),
  }),
  isObject,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  currentStep: AddStandaloneDeviceModalStep;
  choice: AddStandaloneDeviceModalChoice;
  networkOptions: SelectOption<number>[];
  genChoice: WGConfigGenChoice;
  submitSubject: Subject<void>;
  initLocationIpResponse?: GetAvailableLocationIpResponse;
  networks?: Network[];
  genKeys?: {
    publicKey: string;
    privateKey: string;
  };
  manualResponse?: CreateStandaloneDeviceResponse;
  enrollResponse?: StartEnrollmentResponse;
};

type StoreMethods = {
  setStore: (values: Partial<StoreValues>) => void;
  reset: () => void;
  close: () => void;
  open: () => void;
  changeStep: (step: AddStandaloneDeviceModalStep) => void;
};
