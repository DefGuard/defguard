import './style.scss';
import { type ReactNode, useMemo } from 'react';
import { m } from '../../paraglide/messages';
import type {
  WizardPageStep,
  WizardWelcomePageConfig,
} from '../../shared/components/wizard/types';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
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

export const MigrationWizardPage = () => {
  const isWelcome = useMigrationWizardStore((s) => s.isWelcome);
  const activeStep = useMigrationWizardStore((s) => s.activeStep);

  const stepsConfig = useMemo(
    (): Record<MigrationWizardStepValue, WizardPageStep> => ({
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
    }),
    [],
  );

  return (
    <WizardPage
      id="migration-wizard"
      activeStep={activeStep}
      subtitle={m.migration_wizard_subtitle()}
      title={m.migration_wizard_title()}
      steps={stepsConfig}
      isOnWelcomePage={isWelcome}
      welcomePageConfig={welcomePageConfig}
    >
      {stepsComponents[activeStep]}
    </WizardPage>
  );
};
