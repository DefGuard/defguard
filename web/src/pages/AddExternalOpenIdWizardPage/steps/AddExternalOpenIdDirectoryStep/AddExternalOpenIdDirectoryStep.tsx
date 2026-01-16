import { useMutation } from '@tanstack/react-query';
import { cloneDeep } from 'lodash-es';
import { useCallback, useMemo } from 'react';
import type { AddOpenIdProvider } from '../../../../shared/api/types';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import { validateExternalProviderWizard } from '../../consts';
import { useAddExternalOpenIdStore } from '../../useAddExternalOpenIdStore';
import { GoogleProviderForm } from './forms/GoogleProviderForm';
import { JumpcloudProviderForm } from './forms/JumpcloudProviderForm';
import { MicrosoftProviderForm } from './forms/MicrosoftProviderForm';
import { OktaProviderForm } from './forms/OktaProviderForm';

export const AddExternalOpenIdDirectoryStep = () => {
  const provider = useAddExternalOpenIdStore((s) => s.provider);
  const next = useAddExternalOpenIdStore((s) => s.next);

  const { mutateAsync } = useMutation({
    mutationFn: validateExternalProviderWizard,
    onSuccess: (result) => {
      if (typeof result === 'boolean') {
        useAddExternalOpenIdStore.setState({
          testResult: result,
        });
        next();
      } else {
        useAddExternalOpenIdStore.setState({
          testResult: result.success,
          testMessage: result.message,
        });
        next();
      }
    },
    meta: {
      invalidate: [['settings'], ['info'], ['openid', 'provider']],
    },
  });

  const handleValidSubmit = useCallback(
    async (value: Partial<AddOpenIdProvider>) => {
      const state = useAddExternalOpenIdStore.getState()
      const providerState = state.providerState;
      const provider = state.provider;
      const submitValues = { ...cloneDeep(providerState), value, kind: provider };
      await mutateAsync(submitValues);
    },
    [mutateAsync],
  );

  const formRender = useMemo(() => {
    switch (provider) {
      case 'Google':
        return <GoogleProviderForm onSubmit={handleValidSubmit} />;
      case 'Microsoft':
        return <MicrosoftProviderForm onSubmit={handleValidSubmit} />;
      case 'Okta':
        return <OktaProviderForm onSubmit={handleValidSubmit} />;
      case 'JumpCloud':
        return <JumpcloudProviderForm onSubmit={handleValidSubmit} />;
    }
    return null;
  }, [handleValidSubmit, provider]);

  return (
    <WizardCard id="add-external-openid-directory-step">
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
        {
          "You can optionally enable directory synchronization to automatically sync user's status and groups from an external provider."
        }
      </AppText>
      <SizedBox height={ThemeSpacing.Xl} />
      {formRender}
    </WizardCard>
  );
};
