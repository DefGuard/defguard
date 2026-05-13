import { create } from 'zustand';
import {
  type PostureCheckDefguardVersionValue,
  PostureCheckOs,
  type PostureCheckOsValue,
  postureCheckVersionValues,
} from '../PostureChecksPage/types';
import {
  AddPostureCheckWizardStep,
  type AddPostureCheckWizardStepValue,
  addPostureCheckWizardStepOrder,
} from './types';

export type OperatingSystemConditionKey =
  | 'active-directory'
  | 'antivirus'
  | 'disk-encryption'
  | 'device-integrity'
  | 'pre-release';

export type OperatingSystemFormState = {
  conditions: OperatingSystemConditionKey[];
  securityUpdates: boolean;
  version: string;
};

const createDefaultOperatingSystemState = (): Record<
  PostureCheckOsValue,
  OperatingSystemFormState
> => ({
  [PostureCheckOs.Windows]: {
    conditions: [],
    securityUpdates: false,
    version: 'Windows 11',
  },
  [PostureCheckOs.Macos]: {
    conditions: [],
    securityUpdates: false,
    version: postureCheckVersionValues.macos[postureCheckVersionValues.macos.length - 1],
  },
  [PostureCheckOs.Linux]: {
    conditions: [],
    securityUpdates: false,
    version: postureCheckVersionValues.linux[postureCheckVersionValues.linux.length - 1],
  },
  [PostureCheckOs.Ios]: {
    conditions: [],
    securityUpdates: false,
    version: postureCheckVersionValues.ios[postureCheckVersionValues.ios.length - 1],
  },
  [PostureCheckOs.Android]: {
    conditions: [],
    securityUpdates: false,
    version:
      postureCheckVersionValues.android[postureCheckVersionValues.android.length - 1],
  },
});

const createDefaultState = (): StoreValues => ({
  activeStep: AddPostureCheckWizardStep.OperatingSystems,
  allowPrereleaseClient: false,
  configuredOperatingSystems: [],
  description: null,
  minimumClientVersion:
    postureCheckVersionValues.defguard[postureCheckVersionValues.defguard.length - 1],
  name: '',
  operatingSystemState: createDefaultOperatingSystemState(),
});

interface StoreValues {
  activeStep: AddPostureCheckWizardStepValue;
  configuredOperatingSystems: PostureCheckOsValue[];
  allowPrereleaseClient: boolean;
  description: string | null;
  minimumClientVersion: PostureCheckDefguardVersionValue;
  name: string;
  operatingSystemState: Record<PostureCheckOsValue, OperatingSystemFormState>;
}

interface Store extends StoreValues {
  addConfiguredOperatingSystem: (value: PostureCheckOsValue) => void;
  removeConfiguredOperatingSystem: (value: PostureCheckOsValue) => void;
  reset: () => void;
  next: () => void;
  back: () => void;
  setDescription: (value: string | null) => void;
  setAllowPrereleaseClient: (value: boolean) => void;
  setMinimumClientVersion: (value: PostureCheckDefguardVersionValue) => void;
  setName: (value: string) => void;
  updateOperatingSystemDetails: (
    operatingSystem: PostureCheckOsValue,
    values: Partial<OperatingSystemFormState>,
  ) => void;
}

export const useAddPostureCheckWizardStore = create<Store>()((set, get) => ({
  ...createDefaultState(),
  reset: () => set(createDefaultState()),
  addConfiguredOperatingSystem: (value) => {
    if (get().configuredOperatingSystems.includes(value)) {
      return;
    }

    set({
      configuredOperatingSystems: [...get().configuredOperatingSystems, value],
    });
  },
  removeConfiguredOperatingSystem: (value) => {
    set({
      configuredOperatingSystems: get().configuredOperatingSystems.filter(
        (configuredOperatingSystem) => configuredOperatingSystem !== value,
      ),
    });
  },
  next: () => {
    const currentIndex = addPostureCheckWizardStepOrder.indexOf(get().activeStep);
    const nextStep = addPostureCheckWizardStepOrder[currentIndex + 1];

    if (nextStep) {
      set({ activeStep: nextStep });
    }
  },
  back: () => {
    const currentIndex = addPostureCheckWizardStepOrder.indexOf(get().activeStep);
    const previousStep = addPostureCheckWizardStepOrder[currentIndex - 1];

    if (previousStep) {
      set({ activeStep: previousStep });
    }
  },
  setAllowPrereleaseClient: (value) => {
    set({ allowPrereleaseClient: value });
  },
  setDescription: (value) => {
    set({ description: value });
  },
  setMinimumClientVersion: (value) => {
    set({ minimumClientVersion: value });
  },
  setName: (value) => {
    set({ name: value });
  },
  updateOperatingSystemDetails: (operatingSystem, values) => {
    set((state) => ({
      operatingSystemState: {
        ...state.operatingSystemState,
        [operatingSystem]: {
          ...state.operatingSystemState[operatingSystem],
          ...values,
        },
      },
    }));
  },
}));
