import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useEffect, useMemo } from 'react';
import { m } from '../../paraglide/messages';
import type { WizardPageStep } from '../../shared/components/wizard/types';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { SettingsCoreCertificateWizardInternalUrlSettingsStep } from './steps/SettingsCoreCertificateWizardInternalUrlSettingsStep';
import { SettingsCoreCertificateWizardInternalUrlSslConfigStep } from './steps/SettingsCoreCertificateWizardInternalUrlSslConfigStep';
import { SettingsCoreCertificateWizardSummaryStep } from './steps/SettingsCoreCertificateWizardSummaryStep';
import {
  SettingsCoreCertificateWizardStep,
  type SettingsCoreCertificateWizardStepValue,
} from './types';
import { useSettingsCoreCertificateWizardStore } from './useSettingsCoreCertificateWizardStore';

const steps: Record<SettingsCoreCertificateWizardStepValue, ReactNode> = {
  internalUrlSettings: <SettingsCoreCertificateWizardInternalUrlSettingsStep />,
  internalUrlSslConfig: <SettingsCoreCertificateWizardInternalUrlSslConfigStep />,
  summary: <SettingsCoreCertificateWizardSummaryStep />,
};

export const SettingsCoreCertificateWizardPage = () => {
  const activeStep = useSettingsCoreCertificateWizardStore((s) => s.activeStep);
  const navigate = useNavigate();

  useEffect(() => () => useSettingsCoreCertificateWizardStore.getState().reset(), []);

  const stepsConfig = useMemo(
    (): Record<SettingsCoreCertificateWizardStepValue, WizardPageStep> => ({
      internalUrlSettings: {
        id: SettingsCoreCertificateWizardStep.InternalUrlSettings,
        order: 1,
        label: m.settings_certs_core_wizard_step_internal_url_settings_label(),
        description:
          m.settings_certs_core_wizard_step_internal_url_settings_description(),
      },
      internalUrlSslConfig: {
        id: SettingsCoreCertificateWizardStep.InternalUrlSslConfig,
        order: 2,
        label: m.settings_certs_core_wizard_step_internal_url_ssl_config_label(),
        description:
          m.settings_certs_core_wizard_step_internal_url_ssl_config_description(),
      },
      summary: {
        id: SettingsCoreCertificateWizardStep.Summary,
        order: 3,
        label: m.settings_certs_core_wizard_step_summary_label(),
        description: m.settings_certs_core_wizard_step_summary_description(),
      },
    }),
    [],
  );

  return (
    <WizardPage
      id="settings-core-certificate-wizard"
      activeStep={activeStep}
      steps={stepsConfig}
      title={m.settings_certs_core_wizard_title()}
      subtitle={m.settings_certs_core_wizard_subtitle()}
      onClose={() => {
        useSettingsCoreCertificateWizardStore.getState().reset();
        void navigate({ to: '/settings/certs', replace: true });
      }}
    >
      {steps[activeStep]}
    </WizardPage>
  );
};
