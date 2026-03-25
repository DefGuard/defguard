import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import { LocationMfaMode, type LocationMfaModeValue } from '../../../shared/api/types';
import {
  AutoAdoptionSetupStep,
  type AutoAdoptionSetupStepValue,
  type CertInfo,
  type ExternalSslType,
  type InternalSslType,
} from './types';

type StoreValues = {
  activeStep: AutoAdoptionSetupStepValue;
  isAutoAdoptionFlowStarted: boolean;
  isFinishing: boolean;
  admin_first_name: string;
  admin_last_name: string;
  admin_username: string;
  admin_email: string;
  admin_password: string;
  defguard_url: string;
  default_admin_group_name: string;
  default_authentication_period_days: number;
  default_mfa_code_timeout_seconds: number;
  public_proxy_url: string;
  vpn_public_ip: string;
  vpn_wireguard_port: number;
  vpn_gateway_address: string;
  vpn_allowed_ips: string;
  vpn_dns_server_ip: string;
  vpn_mfa_mode: LocationMfaModeValue;
  // Internal URL SSL configuration
  internal_ssl_type: InternalSslType;
  internal_ssl_cert_info: CertInfo | null;
  // External URL SSL configuration
  external_ssl_type: ExternalSslType;
  external_ssl_cert_info: CertInfo | null;
};

type StoreMethods = {
  reset: () => void;
  startFlow: () => void;
  setActiveStep: (step: AutoAdoptionSetupStepValue) => void;
};

const defaults: StoreValues = {
  activeStep: AutoAdoptionSetupStep.AdminUser,
  isAutoAdoptionFlowStarted: false,
  isFinishing: false,
  admin_first_name: '',
  admin_last_name: '',
  admin_username: '',
  admin_email: '',
  admin_password: '',
  defguard_url: '',
  default_admin_group_name: 'admin',
  default_authentication_period_days: 30,
  default_mfa_code_timeout_seconds: 300,
  public_proxy_url: '',
  vpn_public_ip: '',
  vpn_wireguard_port: 51820,
  vpn_gateway_address: '',
  vpn_allowed_ips: '',
  vpn_dns_server_ip: '',
  vpn_mfa_mode: LocationMfaMode.Disabled,
  internal_ssl_type: 'none',
  internal_ssl_cert_info: null,
  external_ssl_type: 'none',
  external_ssl_cert_info: null,
};

export const useAutoAdoptionSetupWizardStore = create<StoreMethods & StoreValues>()(
  persist(
    (set) => ({
      ...defaults,
      reset: () => set({ ...defaults }),
      startFlow: () =>
        set({
          activeStep: AutoAdoptionSetupStep.AdminUser,
          isAutoAdoptionFlowStarted: true,
        }),
      setActiveStep: (step) => set({ activeStep: step }),
    }),
    {
      name: 'auto-adoption-setup-wizard-store',
      storage: createJSONStorage(() => sessionStorage),
      partialize: (state) =>
        omit(state, ['reset', 'startFlow', 'setActiveStep', 'isFinishing']),
    },
  ),
);
