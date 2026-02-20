import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useMemo } from 'react';
import type { WizardPageStep } from '../../shared/components/wizard/types';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { externalProviderName, SUPPORTED_SYNC_PROVIDERS } from '../../shared/constants';
import { AddExternalOpenIdClientSettingsStep } from './steps/AddExternalOpenIdClientSettingsStep/AddExternalOpenIdClientSettingsStep';
import { AddExternalOpenIdDirectoryStep } from './steps/AddExternalOpenIdDirectoryStep/AddExternalOpenIdDirectoryStep';
import { AddExternalOpenIdValidationStep } from './steps/AddExternalOpenIdValidationStep/AddExternalOpenIdValidationStep';
import { AddExternalProviderStep, type AddExternalProviderStepValue } from './types';
import { useAddExternalOpenIdStore } from './useAddExternalOpenIdStore';

const steps: Record<AddExternalProviderStepValue, ReactNode> = {
  'client-settings': <AddExternalOpenIdClientSettingsStep />,
  'directory-sync': <AddExternalOpenIdDirectoryStep />,
  validation: <AddExternalOpenIdValidationStep />,
};

export const AddExternalOpenIdWizardPage = () => {
  const provider = useAddExternalOpenIdStore((s) => s.provider);
  const activeStep = useAddExternalOpenIdStore((s) => s.activeStep);
  const navigate = useNavigate();

  const stepsConfig = useMemo(() => {
    const res: Record<AddExternalProviderStepValue, WizardPageStep> = {
      'client-settings': {
        id: AddExternalProviderStep.ClientSettings,
        label: 'Client Settings',
        order: 0,
        description:
          'Manage core details and connection parameters for your VPN location.',
        hidden: false,
      },
      'directory-sync': {
        id: AddExternalProviderStep.DirectorySync,
        label: 'Directory synchronization',
        order: 1,
        description:
          'Manage core details and connection parameters for your VPN location.',
        hidden: !SUPPORTED_SYNC_PROVIDERS.has(provider),
      },
      validation: {
        id: AddExternalProviderStep.Validation,
        label: 'Validation',
        order: 2,
        description: 'Checking that everything is configured as expected.',
        hidden: false,
      },
    };
    return res;
  }, [provider]);

  return (
    <WizardPage
      activeStep={activeStep}
      steps={stepsConfig}
      title={`${externalProviderName[provider]} external Open ID`}
      subtitle="Configure the OpenID client settings with values provided by your external OpenID provider."
      onClose={() => {
        navigate({
          to: '/settings/openid',
          replace: true,
        }).then(() => {
          useAddExternalOpenIdStore.getState().reset();
        });
      }}
    >
      {steps[activeStep]}
    </WizardPage>
  );
};
