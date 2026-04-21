import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useEffect, useMemo } from 'react';
import { m } from '../../../../paraglide/messages';
import type { WizardPageStep } from '../../../../shared/components/wizard/types';
import { WizardPage } from '../../../../shared/components/wizard/WizardPage/WizardPage';
import { SettingsEdgeCertificateWizardExternalUrlSettingsStep } from './steps/SettingsEdgeCertificateWizardExternalUrlSettingsStep';
import { SettingsEdgeCertificateWizardExternalUrlSslConfigStep } from './steps/SettingsEdgeCertificateWizardExternalUrlSslConfigStep';
import { SettingsEdgeCertificateWizardSummaryStep } from './steps/SettingsEdgeCertificateWizardSummaryStep';
import {
  SettingsEdgeCertificateWizardStep,
  type SettingsEdgeCertificateWizardStepValue,
} from './types';
import { useSettingsEdgeCertificateWizardStore } from './useSettingsEdgeCertificateWizardStore';

const steps: Record<SettingsEdgeCertificateWizardStepValue, ReactNode> = {
  externalUrlSettings: <SettingsEdgeCertificateWizardExternalUrlSettingsStep />,
  externalUrlSslConfig: <SettingsEdgeCertificateWizardExternalUrlSslConfigStep />,
  summary: <SettingsEdgeCertificateWizardSummaryStep />,
};

export const SettingsEdgeCertificateWizardPage = () => {
  const activeStep = useSettingsEdgeCertificateWizardStore((s) => s.activeStep);
  const navigate = useNavigate();

  useEffect(() => () => useSettingsEdgeCertificateWizardStore.getState().reset(), []);

  const stepsConfig = useMemo(
    (): Record<SettingsEdgeCertificateWizardStepValue, WizardPageStep> => ({
      externalUrlSettings: {
        id: SettingsEdgeCertificateWizardStep.ExternalUrlSettings,
        order: 1,
        label: m.settings_certs_edge_wizard_step_external_url_settings_label(),
        description:
          m.settings_certs_edge_wizard_step_external_url_settings_description(),
      },
      externalUrlSslConfig: {
        id: SettingsEdgeCertificateWizardStep.ExternalUrlSslConfig,
        order: 2,
        label: m.settings_certs_edge_wizard_step_external_url_ssl_config_label(),
        description:
          m.settings_certs_edge_wizard_step_external_url_ssl_config_description(),
      },
      summary: {
        id: SettingsEdgeCertificateWizardStep.Summary,
        order: 3,
        label: m.settings_certs_edge_wizard_step_summary_label(),
        description: m.settings_certs_edge_wizard_step_summary_description(),
      },
    }),
    [],
  );

  return (
    <WizardPage
      id="settings-edge-certificate-wizard"
      activeStep={activeStep}
      steps={stepsConfig}
      title={m.settings_certs_edge_wizard_title()}
      subtitle={m.settings_certs_edge_wizard_subtitle()}
      onClose={() => {
        useSettingsEdgeCertificateWizardStore.getState().reset();
        void navigate({ to: '/settings/certs', replace: true });
      }}
    >
      {steps[activeStep]}
    </WizardPage>
  );
};
