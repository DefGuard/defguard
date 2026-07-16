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
  /** Windows only */
  securityUpdateMaxAge: number | null;
  /** Android only */
  androidSecurityPatchLevelMaxAge: number | null;
  version: PostureCheckOsVersionValue | null;
};

const getCurrentVersionOrAny = <T extends string | number>(
  values: readonly T[],
  currentValue?: T | null,
): T | null => {
  if (!isPresent(currentValue)) {
    return null;
  }

  if (values.includes(currentValue)) {
    return currentValue;
  }

  return null;
};

const emptyPostureCheckVersionValues: PostureCheckVersionValues = {
  windows: [],
  macos: [],
  linux: [],
  ios: [],
  android: [],
  defguardDesktop: [],
  defguardMobile: [],
};

const createDefaultOperatingSystemState = (
  versionValues: PostureCheckVersionValues,
): Record<PostureCheckOsValue, OperatingSystemFormState> => ({
  [PostureCheckOs.Windows]: {
    conditions: [],
    securityUpdateMaxAge: null,
    androidSecurityPatchLevelMaxAge: null,
    version: getCurrentVersionOrAny(versionValues.windows),
  },
  [PostureCheckOs.Macos]: {
    conditions: [],
    securityUpdateMaxAge: null,
    androidSecurityPatchLevelMaxAge: null,
    version: getCurrentVersionOrAny(versionValues.macos),
  },
  [PostureCheckOs.Linux]: {
    conditions: [],
    securityUpdateMaxAge: null,
    androidSecurityPatchLevelMaxAge: null,
    version: getCurrentVersionOrAny(versionValues.linux),
  },
  [PostureCheckOs.Ios]: {
    conditions: [],
    securityUpdateMaxAge: null,
    androidSecurityPatchLevelMaxAge: null,
    version: getCurrentVersionOrAny(versionValues.ios),
  },
  [PostureCheckOs.Android]: {
    conditions: [],
    securityUpdateMaxAge: null,
    androidSecurityPatchLevelMaxAge: null,
    version: getCurrentVersionOrAny(versionValues.android),
  },
});

const createDefaultState = (versionValues: PostureCheckVersionValues): StoreValues => ({
  activeStep: AddPostureCheckWizardStep.OperatingSystems,
  allowPrereleaseClient: false,
  configuredOperatingSystems: [],
  description: null,
  minimumDesktopClientVersion: getCurrentVersionOrAny(versionValues.defguardDesktop),
  minimumMobileClientVersion: getCurrentVersionOrAny(versionValues.defguardMobile),
  name: '',
  operatingSystemState: createDefaultOperatingSystemState(versionValues),
  availableVersionValues: versionValues,
});

interface StoreValues {
  activeStep: AddPostureCheckWizardStepValue;
  configuredOperatingSystems: PostureCheckOsValue[];
  allowPrereleaseClient: boolean;
  description: string | null;
  minimumDesktopClientVersion: PostureCheckDefguardVersionValue;
  minimumMobileClientVersion: PostureCheckDefguardVersionValue;
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
  setMinimumDesktopClientVersion: (value: PostureCheckDefguardVersionValue) => void;
  setMinimumMobileClientVersion: (value: PostureCheckDefguardVersionValue) => void;
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
      minimumDesktopClientVersion: getCurrentVersionOrAny(
        versionValues.defguardDesktop,
        state.minimumDesktopClientVersion,
      ),
      minimumMobileClientVersion: getCurrentVersionOrAny(
        versionValues.defguardMobile,
        state.minimumMobileClientVersion,
      ),
      operatingSystemState: {
        [PostureCheckOs.Windows]: {
          ...state.operatingSystemState[PostureCheckOs.Windows],
          version: getCurrentVersionOrAny(
            versionValues.windows,
            state.operatingSystemState[PostureCheckOs.Windows].version,
          ),
        },
        [PostureCheckOs.Macos]: {
          ...state.operatingSystemState[PostureCheckOs.Macos],
          version: getCurrentVersionOrAny(
            versionValues.macos,
            state.operatingSystemState[PostureCheckOs.Macos].version,
          ),
        },
        [PostureCheckOs.Linux]: {
          ...state.operatingSystemState[PostureCheckOs.Linux],
          version: getCurrentVersionOrAny(
            versionValues.linux,
            state.operatingSystemState[PostureCheckOs.Linux].version,
          ),
        },
        [PostureCheckOs.Ios]: {
          ...state.operatingSystemState[PostureCheckOs.Ios],
          version: getCurrentVersionOrAny(
            versionValues.ios,
            state.operatingSystemState[PostureCheckOs.Ios].version,
          ),
        },
        [PostureCheckOs.Android]: {
          ...state.operatingSystemState[PostureCheckOs.Android],
          version: getCurrentVersionOrAny(
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
  setMinimumDesktopClientVersion: (value) => {
    set({ minimumDesktopClientVersion: value });
  },
  setMinimumMobileClientVersion: (value) => {
    set({ minimumMobileClientVersion: value });
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
