import { isObject } from 'lodash-es';
import { createWithEqualityFn } from 'zustand/traditional';

import { AddStandaloneDeviceModalChoice, AddStandaloneDeviceModalStep } from './types';

const defaultValues: StoreValues = {
  visible: false,
  currentStep: AddStandaloneDeviceModalStep.METHOD_CHOICE,
  choice: AddStandaloneDeviceModalChoice.CLI,
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
};

type StoreMethods = {
  setStore: (values: Partial<StoreValues>) => void;
  reset: () => void;
  close: () => void;
  open: () => void;
  changeStep: (step: AddStandaloneDeviceModalStep) => void;
};
