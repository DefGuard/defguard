import { create } from 'zustand';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import {
  type PostureCheckDefguardVersionValue,
  PostureCheckOs,
  type PostureCheckOsValue,
  type PostureCheckOsVersionValue,
  type PostureCheckVersionValues,
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
  version: PostureCheckOsVersionValue | null;
};

const getCurrentOrLatestVersion = <T extends string | number>(
  values: readonly T[],
  currentValue?: T | null,
): T | null => {
  if (isPresent(currentValue) && values.includes(currentValue)) {
    return currentValue;
  }

  return values[values.length - 1] ?? currentValue ?? null;
};

const emptyPostureCheckVersionValues: PostureCheckVersionValues = {
  windows: [],
  macos: [],
  linux: [],
  ios: [],
  android: [],
  defguard: [],
};

const createDefaultOperatingSystemState = (
  versionValues: PostureCheckVersionValues,
): Record<PostureCheckOsValue, OperatingSystemFormState> => ({
  [PostureCheckOs.Windows]: {
    conditions: [],
    securityUpdates: false,
    version: getCurrentOrLatestVersion(versionValues.windows),
  },
  [PostureCheckOs.Macos]: {
    conditions: [],
    securityUpdates: false,
    version: getCurrentOrLatestVersion(versionValues.macos),
  },
  [PostureCheckOs.Linux]: {
    conditions: [],
    securityUpdates: false,
    version: getCurrentOrLatestVersion(versionValues.linux),
  },
  [PostureCheckOs.Ios]: {
    conditions: [],
    securityUpdates: false,
    version: getCurrentOrLatestVersion(versionValues.ios),
  },
  [PostureCheckOs.Android]: {
    conditions: [],
    securityUpdates: false,
    version: getCurrentOrLatestVersion(versionValues.android),
  },
});

const createDefaultState = (versionValues: PostureCheckVersionValues): StoreValues => ({
  activeStep: AddPostureCheckWizardStep.OperatingSystems,
  allowPrereleaseClient: false,
  configuredOperatingSystems: [],
  description: null,
  minimumClientVersion: getCurrentOrLatestVersion(versionValues.defguard) ?? '',
  name: '',
  operatingSystemState: createDefaultOperatingSystemState(versionValues),
  availableVersionValues: versionValues,
});

interface StoreValues {
  activeStep: AddPostureCheckWizardStepValue;
  configuredOperatingSystems: PostureCheckOsValue[];
  allowPrereleaseClient: boolean;
  description: string | null;
  minimumClientVersion: PostureCheckDefguardVersionValue;
  name: string;
  operatingSystemState: Record<PostureCheckOsValue, OperatingSystemFormState>;
  availableVersionValues: PostureCheckVersionValues;
}

interface Store extends StoreValues {
  addConfiguredOperatingSystem: (value: PostureCheckOsValue) => void;
  removeConfiguredOperatingSystem: (value: PostureCheckOsValue) => void;
  reset: () => void;
  next: () => void;
  back: () => void;
  syncVersionValues: (versionValues: PostureCheckVersionValues) => void;
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
  ...createDefaultState(emptyPostureCheckVersionValues),
  reset: () => set(createDefaultState(get().availableVersionValues)),
  syncVersionValues: (versionValues) => {
    set((state) => ({
      availableVersionValues: versionValues,
      minimumClientVersion:
        getCurrentOrLatestVersion(versionValues.defguard, state.minimumClientVersion) ??
        '',
      operatingSystemState: {
        [PostureCheckOs.Windows]: {
          ...state.operatingSystemState[PostureCheckOs.Windows],
          version: getCurrentOrLatestVersion(
            versionValues.windows,
            state.operatingSystemState[PostureCheckOs.Windows].version,
          ),
        },
        [PostureCheckOs.Macos]: {
          ...state.operatingSystemState[PostureCheckOs.Macos],
          version: getCurrentOrLatestVersion(
            versionValues.macos,
            state.operatingSystemState[PostureCheckOs.Macos].version,
          ),
        },
        [PostureCheckOs.Linux]: {
          ...state.operatingSystemState[PostureCheckOs.Linux],
          version: getCurrentOrLatestVersion(
            versionValues.linux,
            state.operatingSystemState[PostureCheckOs.Linux].version,
          ),
        },
        [PostureCheckOs.Ios]: {
          ...state.operatingSystemState[PostureCheckOs.Ios],
          version: getCurrentOrLatestVersion(
            versionValues.ios,
            state.operatingSystemState[PostureCheckOs.Ios].version,
          ),
        },
        [PostureCheckOs.Android]: {
          ...state.operatingSystemState[PostureCheckOs.Android],
          version: getCurrentOrLatestVersion(
            versionValues.android,
            state.operatingSystemState[PostureCheckOs.Android].version,
          ),
        },
      },
    }));
  },
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
