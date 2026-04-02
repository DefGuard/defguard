import { create } from 'zustand';
import type { CertInfo, ExternalSslType } from '../../shared/api/types';
import {
  SettingsEdgeCertificateWizardStep,
  type SettingsEdgeCertificateWizardStepValue,
} from './types';

type StoreValues = {
  activeStep: SettingsEdgeCertificateWizardStepValue;
  external_ssl_type: ExternalSslType | null;
  external_ssl_cert_info: CertInfo | null;
};

type Store = StoreValues & {
  setState: (values: Partial<StoreValues>) => void;
  reset: () => void;
  next: () => void;
  back: () => void;
};

const defaults: StoreValues = {
  activeStep: SettingsEdgeCertificateWizardStep.ExternalUrlSettings,
  external_ssl_type: null,
  external_ssl_cert_info: null,
};

export const useSettingsEdgeCertificateWizardStore = create<Store>()((set, get) => ({
  ...defaults,
  setState: (values) => set(values),
  reset: () => set(defaults),
  next: () => {
    const activeStep = get().activeStep;
    if (activeStep === SettingsEdgeCertificateWizardStep.ExternalUrlSettings) {
      set({ activeStep: SettingsEdgeCertificateWizardStep.ExternalUrlSslConfig });
      return;
    }
    if (activeStep === SettingsEdgeCertificateWizardStep.ExternalUrlSslConfig) {
      set({ activeStep: SettingsEdgeCertificateWizardStep.Summary });
    }
  },
  back: () => {
    const activeStep = get().activeStep;
    if (activeStep === SettingsEdgeCertificateWizardStep.Summary) {
      set({ activeStep: SettingsEdgeCertificateWizardStep.ExternalUrlSslConfig });
      return;
    }
    if (activeStep === SettingsEdgeCertificateWizardStep.ExternalUrlSslConfig) {
      set({ activeStep: SettingsEdgeCertificateWizardStep.ExternalUrlSettings });
    }
  },
}));
