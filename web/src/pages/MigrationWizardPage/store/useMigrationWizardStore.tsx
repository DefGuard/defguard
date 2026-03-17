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
import { getMigrationStateQueryOptions } from '../../../shared/query';
import type { EdgeAdoptionState } from '../../EdgeSetupPage/types';
import {
  type CAOptionType,
  MigrationWizardStep,
  type MigrationWizardStepValue,
} from '../types';

interface StoreValues extends MigrationWizardApiState {
  // general config
  defguard_url: string;
  public_proxy_url: string;
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
  ca_common_name: m.migration_wizard_ca_placeholder_common_name(),
  ca_email: '',
  ca_validity_period_years: 5,
  ca_cert_file: null,
  ca_option: null,
  common_name: '',
  ip_or_domain: '',
  grpc_port: 50051,
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
