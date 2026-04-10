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
  getLocationsCountQueryOptions,
  getMigrationStateQueryOptions,
  getSettingsQueryOptions,
} from '../../shared/query';
import { MigrationWizardCAStep } from './steps/MigrationWizardCAStep';
import { MigrationWizardCASummaryStep } from './steps/MigrationWizardCASummaryStep';
import { MigrationWizardConfirmationStep } from './steps/MigrationWizardConfirmationStep/MigrationWizardConfirmationStep';
import { MigrationWizardEdgeAdoptionStep } from './steps/MigrationWizardEdgeAdoptionStep';
import { MigrationWizardEdgeComponentStep } from './steps/MigrationWizardEdgeComponentStep';
import { MigrationWizardEdgeDeploymentStepAdapter } from './steps/MigrationWizardEdgeDeploymentStepAdapter';
import { MigrationWizardExternalUrlSettingsStep } from './steps/MigrationWizardExternalUrlSettingsStep';
import { MigrationWizardExternalUrlSslConfigStep } from './steps/MigrationWizardExternalUrlSslConfigStep';
import { MigrationWizardGeneralConfigurationStep } from './steps/MigrationWizardGeneralConfigurationStep';
import { MigrationWizardInternalUrlSettingsStep } from './steps/MigrationWizardInternalUrlSettingsStep';
import { MigrationWizardInternalUrlSslConfigStep } from './steps/MigrationWizardInternalUrlSslConfigStep';
import { MigrationWizardStart } from './steps/MigrationWizardStart';
import { useMigrationWizardStore } from './store/useMigrationWizardStore';
import { MigrationWizardStep, type MigrationWizardStepValue } from './types';

type ConfigurableSteps = Exclude<MigrationWizardStepValue, 'welcome'>;

export const MigrationWizardPage = () => {
  const { data: locationCount } = useSuspenseQuery(getLocationsCountQueryOptions);
  const { data: wizardState } = useSuspenseQuery(getMigrationStateQueryOptions);
  const { data: settings } = useSuspenseQuery(getSettingsQueryOptions);

  const activeStep = useMigrationWizardStore((s) => s.current_step);

  const welcomePageConfig = useMemo(
    (): WizardWelcomePageConfig =>
      ({
        title: m.migration_wizard_welcome_title(),
        subtitle: m.migration_wizard_welcome_subtitle({ count: locationCount }),
        content: <MigrationWizardStart />,
        docsText: m.migration_wizard_welcome_docs_text(),
      }) as const,
    [locationCount],
  );

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
      edgeDeployment: {
        id: MigrationWizardStep.EdgeDeployment,
        order: 4,
        label: m.edge_setup_step_deploy_label(),
        description: m.migration_wizard_step_edge_deploy_description(),
      },
      edge: {
        id: MigrationWizardStep.Edge,
        order: 5,
        label: m.migration_wizard_step_edge_component_label(),
        description: m.migration_wizard_step_edge_component_description(),
      },
      edgeAdoption: {
        id: MigrationWizardStep.EdgeAdoption,
        order: 6,
        label: m.migration_wizard_step_edge_adoption_label(),
        description: m.migration_wizard_step_edge_adoption_description(),
      },
      internalUrlSettings: {
        id: MigrationWizardStep.InternalUrlSettings,
        order: 7,
        label: m.migration_wizard_step_internal_url_settings_label(),
        description: m.migration_wizard_step_internal_url_settings_description(),
      },
      internalUrlSslConfig: {
        id: MigrationWizardStep.InternalUrlSslConfig,
        order: 8,
        label: m.migration_wizard_step_internal_url_ssl_config_label(),
        description: m.migration_wizard_step_internal_url_ssl_config_description(),
      },
      externalUrlSettings: {
        id: MigrationWizardStep.ExternalUrlSettings,
        order: 9,
        label: m.migration_wizard_step_external_url_settings_label(),
        description: m.migration_wizard_step_external_url_settings_description(),
      },
      externalUrlSslConfig: {
        id: MigrationWizardStep.ExternalUrlSslConfig,
        order: 10,
        label: m.migration_wizard_step_external_url_ssl_config_label(),
        description: m.migration_wizard_step_external_url_ssl_config_description(),
      },
      confirmation: {
        id: MigrationWizardStep.Confirmation,
        order: 11,
        label: m.migration_wizard_step_confirmation_label(),
        description: m.migration_wizard_step_confirmation_description(),
      },
    }),
    [],
  );

  const stepsComponents = useMemo(
    (): Record<MigrationWizardStepValue, ReactNode> => ({
      general: <MigrationWizardGeneralConfigurationStep />,
      internalUrlSettings: <MigrationWizardInternalUrlSettingsStep />,
      internalUrlSslConfig: <MigrationWizardInternalUrlSslConfigStep />,
      externalUrlSettings: <MigrationWizardExternalUrlSettingsStep />,
      externalUrlSslConfig: <MigrationWizardExternalUrlSslConfigStep />,
      ca: <MigrationWizardCAStep />,
      caSummary: <MigrationWizardCASummaryStep />,
      edge: <MigrationWizardEdgeComponentStep />,
      edgeDeployment: <MigrationWizardEdgeDeploymentStepAdapter />,
      edgeAdoption: <MigrationWizardEdgeAdoptionStep />,
      confirmation: <MigrationWizardConfirmationStep />,
      welcome: null,
    }),
    [],
  );

  // sync wizard state
  useEffect(() => {
    if (wizardState) {
      useMigrationWizardStore.setState({
        ...wizardState,
        ...(wizardState.proxy_url && {
          ip_or_domain: wizardState.proxy_url.domain,
          grpc_port: wizardState.proxy_url.port,
        }),
      });
    }
  }, [wizardState]);

  // sync settings state
  useEffect(() => {
    if (settings) {
      useMigrationWizardStore.setState({
        defguard_url: settings.defguard_url,
        public_proxy_url: settings.public_proxy_url,
        default_admin_group_name: settings.default_admin_group_name,
        authentication_period_days: settings.authentication_period_days,
        mfa_code_timeout_seconds: settings.mfa_code_timeout_seconds,
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
      videoGuidePlacementKey="migrationWizard"
      isOnWelcomePage={activeStep === MigrationWizardStep.Welcome}
      welcomePageConfig={welcomePageConfig}
    >
      {stepsComponents[activeStep]}
    </WizardPage>
  );
};
