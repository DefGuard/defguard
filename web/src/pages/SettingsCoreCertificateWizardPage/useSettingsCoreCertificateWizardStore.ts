import { create } from 'zustand';
import type { CertInfo, InternalSslType } from '../../shared/api/types';
import {
  SettingsCoreCertificateWizardStep,
  type SettingsCoreCertificateWizardStepValue,
} from './types';

type StoreValues = {
  activeStep: SettingsCoreCertificateWizardStepValue;
  internal_ssl_type: InternalSslType | null;
  internal_ssl_cert_info: CertInfo | null;
};

type Store = StoreValues & {
  setState: (values: Partial<StoreValues>) => void;
  reset: () => void;
  next: () => void;
  back: () => void;
};

const defaults: StoreValues = {
  activeStep: SettingsCoreCertificateWizardStep.InternalUrlSettings,
  internal_ssl_type: null,
  internal_ssl_cert_info: null,
};

export const useSettingsCoreCertificateWizardStore = create<Store>()((set, get) => ({
  ...defaults,
  setState: (values) => set(values),
  reset: () => set(defaults),
  next: () => {
    const activeStep = get().activeStep;
    if (activeStep === SettingsCoreCertificateWizardStep.InternalUrlSettings) {
      set({ activeStep: SettingsCoreCertificateWizardStep.InternalUrlSslConfig });
      return;
    }
    if (activeStep === SettingsCoreCertificateWizardStep.InternalUrlSslConfig) {
      set({ activeStep: SettingsCoreCertificateWizardStep.Summary });
    }
  },
  back: () => {
    const activeStep = get().activeStep;
    if (activeStep === SettingsCoreCertificateWizardStep.Summary) {
      set({ activeStep: SettingsCoreCertificateWizardStep.InternalUrlSslConfig });
      return;
    }
    if (activeStep === SettingsCoreCertificateWizardStep.InternalUrlSslConfig) {
      set({ activeStep: SettingsCoreCertificateWizardStep.InternalUrlSettings });
    }
  },
}));
