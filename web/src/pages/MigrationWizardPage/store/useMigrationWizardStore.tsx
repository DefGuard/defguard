import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import { queryClient } from '../../../app/query';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type {
  MigrationWizardApiState,
  MigrationWizardLocationState,
} from '../../../shared/api/types';
import { edgeDefaultGrpcPort } from '../../../shared/constants';
import { getMigrationStateQueryOptions } from '../../../shared/query';
import type { EdgeAdoptionState } from '../../EdgeSetupPage/types';
import type {
  CertInfo,
  ExternalSslType,
  InternalSslType,
} from '../../SetupPage/autoAdoption/types';
import {
  type CAOptionType,
  MigrationWizardStep,
  type MigrationWizardStepValue,
} from '../types';

interface StoreValues extends MigrationWizardApiState {
  // general config
  defguard_url: string;
  public_proxy_url: string;
  default_admin_group_name: string;
  authentication_period_days: number;
  mfa_code_timeout_seconds: number;
  // internal URL SSL configuration
  internal_ssl_type: InternalSslType | null;
  internal_ssl_cert_info: CertInfo | null;
  // external URL SSL configuration
  external_ssl_type: ExternalSslType | null;
  external_ssl_cert_info: CertInfo | null;
  // ca
  ca_common_name: string;
  ca_email: string;
  ca_validity_period_years: number;
  ca_cert_file: File | null;
  ca_option: CAOptionType | null;
  // edge config
  common_name: string;
  ip_or_domain: string;
  grpc_port: number;
  edgeAdoptionState: EdgeAdoptionState;
}

const edgeAdoptionStateDefaults: EdgeAdoptionState = {
  isProcessing: false,
  isComplete: false,
  currentStep: null,
  errorMessage: null,
  proxyVersion: null,
  proxyLogs: [],
};

const defaults: StoreValues = {
  current_step: MigrationWizardStep.General,
  location_state: null,
  defguard_url: '',
  public_proxy_url: '',
  default_admin_group_name: 'admin',
  authentication_period_days: 30,
  mfa_code_timeout_seconds: 300,
  internal_ssl_type: null,
  internal_ssl_cert_info: null,
  external_ssl_type: null,
  external_ssl_cert_info: null,
  ca_common_name: m.migration_wizard_ca_placeholder_common_name(),
  ca_email: '',
  ca_validity_period_years: 5,
  ca_cert_file: null,
  ca_option: null,
  common_name: '',
  ip_or_domain: '',
  grpc_port: edgeDefaultGrpcPort,
  edgeAdoptionState: edgeAdoptionStateDefaults,
};

interface Store extends StoreValues {
  setState: (values: Partial<StoreValues>) => void;
  resetEdgeAdoptionState: () => void;
  setEdgeAdoptionState: (state: Partial<EdgeAdoptionState>) => void;
  resetState: () => void;
  next: () => void;
  back: () => void;
}

const saveStep = (
  step: MigrationWizardStepValue,
  locationState: MigrationWizardLocationState | null,
) => {
  void api.migration.state
    .updateMigrationState({
      current_step: step,
      location_state: locationState,
    })
    .then(() => {
      void queryClient
        .invalidateQueries({
          queryKey: getMigrationStateQueryOptions.queryKey,
        })
        .catch(() => {
          console.error(`Failed to invalidate migration wizard state query key.`);
        });
    })
    .catch(() => {
      console.error(`Failed to save migration state`);
    });
};

export const useMigrationWizardStore = create<Store>()(
  persist(
    (set, get) => ({
      ...defaults,
      setState: (newValues) => {
        set(newValues);
      },
      resetEdgeAdoptionState: () =>
        set(() => ({
          edgeAdoptionState: { ...edgeAdoptionStateDefaults },
        })),
      setEdgeAdoptionState: (state: Partial<EdgeAdoptionState>) =>
        set((s) => ({
          edgeAdoptionState: { ...s.edgeAdoptionState, ...state },
        })),
      resetState: () => {
        set(defaults);
      },
      back: () => {
        const current = get().current_step;
        const locationState = get().location_state;
        let stepToSet: MigrationWizardStepValue | null;
        switch (current) {
          case 'welcome':
            stepToSet = null;
            break;
          case 'general':
            stepToSet = MigrationWizardStep.Welcome;
            break;
          case 'ca':
            stepToSet = MigrationWizardStep.General;
            break;
          case 'caSummary':
            stepToSet = MigrationWizardStep.Ca;
            break;
          case 'edgeDeployment':
            stepToSet = MigrationWizardStep.CaSummary;
            break;
          case 'edge':
            stepToSet = MigrationWizardStep.EdgeDeployment;
            break;
          case 'edgeAdoption':
            stepToSet = MigrationWizardStep.Edge;
            break;
          case 'internalUrlSettings':
            stepToSet = MigrationWizardStep.EdgeAdoption;
            break;
          case 'internalUrlSslConfig':
            stepToSet = MigrationWizardStep.InternalUrlSettings;
            break;
          case 'externalUrlSettings':
            stepToSet = MigrationWizardStep.InternalUrlSslConfig;
            break;
          case 'externalUrlSslConfig':
            stepToSet = MigrationWizardStep.ExternalUrlSettings;
            break;
          case 'confirmation':
            stepToSet = null;
            break;
        }
        if (stepToSet) {
          set({
            current_step: stepToSet,
          });
          saveStep(stepToSet, locationState);
        }
      },
      next: () => {
        const current = get().current_step;
        let stepToSet: MigrationWizardStepValue | null;
        switch (current) {
          case 'welcome':
            stepToSet = MigrationWizardStep.General;
            break;
          case 'general':
            stepToSet = MigrationWizardStep.Ca;
            break;
          case 'ca':
            stepToSet = MigrationWizardStep.CaSummary;
            break;
          case 'caSummary':
            stepToSet = MigrationWizardStep.EdgeDeployment;
            break;
          case 'edgeDeployment':
            stepToSet = MigrationWizardStep.Edge;
            break;
          case 'edge':
            stepToSet = MigrationWizardStep.EdgeAdoption;
            break;
          case 'edgeAdoption':
            stepToSet = MigrationWizardStep.InternalUrlSettings;
            break;
          case 'internalUrlSettings':
            stepToSet = MigrationWizardStep.InternalUrlSslConfig;
            break;
          case 'internalUrlSslConfig':
            stepToSet = MigrationWizardStep.ExternalUrlSettings;
            break;
          case 'externalUrlSettings':
            stepToSet = MigrationWizardStep.ExternalUrlSslConfig;
            break;
          case 'externalUrlSslConfig':
            stepToSet = MigrationWizardStep.Confirmation;
            break;
          case 'confirmation':
            stepToSet = null;
            break;
        }
        if (stepToSet) {
          set({
            current_step: stepToSet,
          });
          saveStep(stepToSet, get().location_state);
        }
      },
    }),
    {
      name: 'migration-wizard',
      storage: createJSONStorage(() => sessionStorage),
      partialize: (state) =>
        omit(state, [
          'setState',
          'resetEdgeAdoptionState',
          'setEdgeAdoptionState',
          'resetState',
          'next',
          'back',
        ]),
    },
  ),
);
