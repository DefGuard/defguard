import './style.scss';
import { useSuspenseQuery } from '@tanstack/react-query';
import { type ReactNode, useEffect, useMemo } from 'react';
import { m } from '../../paraglide/messages';
import type {
  WizardPageStep,
  WizardWelcomePageConfig,
} from '../../shared/components/wizard/types';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import {
  getMigrationStateQueryOptions,
  getSettingsQueryOptions,
} from '../../shared/query';
import { MigrationWizardCAStep } from './steps/MigrationWizardCAStep';
import { MigrationWizardCASummaryStep } from './steps/MigrationWizardCASummaryStep';
import { MigrationWizardConfirmationStep } from './steps/MigrationWizardConfirmationStep/MigrationWizardConfirmationStep';
import { MigrationWizardEdgeAdoptionStep } from './steps/MigrationWizardEdgeAdoptionStep';
import { MigrationWizardEdgeComponentStep } from './steps/MigrationWizardEdgeComponentStep';
import { MigrationWizardGeneralConfigurationStep } from './steps/MigrationWizardGeneralConfigurationStep';
import { MigrationWizardStart } from './steps/MigrationWizardStart';
import { useMigrationWizardStore } from './store/useMigrationWizardStore';
import { MigrationWizardStep, type MigrationWizardStepValue } from './types';

const welcomePageConfig: WizardWelcomePageConfig = {
  title: 'Welcome to Defguard Migration Wizard.',
  subtitle: `We've detected your previous version 1.X so email.`,
  content: <MigrationWizardStart />,
  docsText: `We'll guide you through the process step by step. For full details, see the migration guide following the link bellow.`,
} as const;

type ConfigurableSteps = Exclude<MigrationWizardStepValue, 'welcome'>;

export const MigrationWizardPage = () => {
  const { data: wizardState } = useSuspenseQuery(getMigrationStateQueryOptions);
  const { data: settings } = useSuspenseQuery(getSettingsQueryOptions);

  const activeStep = useMigrationWizardStore((s) => s.current_step);

  const stepsConfig = useMemo(
    (): Record<ConfigurableSteps, WizardPageStep> => ({
      general: {
        id: MigrationWizardStep.General,
        order: 1,
        label: m.migration_wizard_step_general_config_label(),
        description: m.migration_wizard_step_general_config_description(),
      },
      ca: {
        id: MigrationWizardStep.Ca,
        order: 2,
        label: m.migration_wizard_step_certificate_authority_label(),
        description: m.migration_wizard_step_certificate_authority_description(),
      },
      caSummary: {
        id: MigrationWizardStep.CaSummary,
        order: 3,
        label: m.migration_wizard_step_certificate_authority_summary_label(),
        description: m.migration_wizard_step_certificate_authority_summary_description(),
      },
      edge: {
        id: MigrationWizardStep.Edge,
        order: 4,
        label: m.migration_wizard_step_edge_component_label(),
        description: m.migration_wizard_step_edge_component_description(),
      },
      edgeAdoption: {
        id: MigrationWizardStep.EdgeAdoption,
        order: 5,
        label: m.migration_wizard_step_edge_adoption_label(),
        description: m.migration_wizard_step_edge_adoption_description(),
      },
      confirmation: {
        id: MigrationWizardStep.Confirmation,
        order: 6,
        label: m.migration_wizard_step_confirmation_label(),
        description: m.migration_wizard_step_confirmation_description(),
      },
    }),
    [],
  );

  const stepsComponents = useMemo(
    (): Record<MigrationWizardStepValue, ReactNode> => ({
      general: <MigrationWizardGeneralConfigurationStep />,
      ca: <MigrationWizardCAStep />,
      caSummary: <MigrationWizardCASummaryStep />,
      edge: <MigrationWizardEdgeComponentStep />,
      edgeAdoption: <MigrationWizardEdgeAdoptionStep />,
      confirmation: <MigrationWizardConfirmationStep />,
      welcome: null,
    }),
    [],
  );

  // sync wizard state
  useEffect(() => {
    if (wizardState) {
      useMigrationWizardStore.setState(wizardState);
    }
  }, [wizardState]);

  // sync settings state
  useEffect(() => {
    if (settings) {
      useMigrationWizardStore.setState({
        defguard_url: settings.defguard_url,
        default_admin_group_name: settings.default_admin_group_name,
        default_authentication_period_days: settings.authentication_period_days,
        default_mfa_code_timeout_seconds: settings.mfa_code_timeout_seconds,
        public_proxy_url: settings.public_proxy_url,
      });
    }
  }, [settings]);

  return (
    <WizardPage
      id="migration-wizard"
      activeStep={activeStep}
      subtitle={m.migration_wizard_subtitle()}
      title={m.migration_wizard_title()}
      steps={stepsConfig}
      isOnWelcomePage={activeStep === MigrationWizardStep.Welcome}
      welcomePageConfig={welcomePageConfig}
    >
      {stepsComponents[activeStep]}
    </WizardPage>
  );
};
