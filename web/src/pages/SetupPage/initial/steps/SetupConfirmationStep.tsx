import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { ActionCard } from '../../../../shared/components/ActionCard/ActionCard';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { Icon } from '../../../../shared/defguard-ui/components/Icon';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import location from '../../assets/location.png';
import { useSetupWizardStore } from '../useSetupWizardStore';

export const SetupConfirmationStep = () => {
  const navigate = useNavigate();
  const [isSubmitting, setIsSubmitting] = useState(false);

  const waitForSettingsEssentials = async ({
    timeoutMs = 60_000,
    intervalMs = 500,
  }: {
    timeoutMs?: number;
    intervalMs?: number;
  }) => {
    const startedAt = Date.now();

    while (Date.now() - startedAt < timeoutMs) {
      try {
        const response = await api.getSessionInfo();

        if (isPresent(response.data) && response.data.active_wizard === null) {
          return;
        }
      } catch (_error) {
        // Ignore errors while API restarts.
      }

      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }

    throw new Error('Timed out waiting for settings essentials.');
  };

  const handleFinish = async () => {
    try {
      setIsSubmitting(true);
      useSetupWizardStore.setState({ isFinishing: true });
      await finishSetup();
      await waitForSettingsEssentials({});
      await navigate({ to: '/add-location', replace: true });
      setTimeout(() => {
        useSetupWizardStore.getState().reset();
      }, 100);
    } catch (error) {
      console.error('Failed to finish setup flow:', error);
      useSetupWizardStore.setState({ isFinishing: false });
      Snackbar.error(m.initial_setup_confirmation_error_finish_failed());
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleExit = async () => {
    try {
      setIsSubmitting(true);
      useSetupWizardStore.setState({ isFinishing: true });
      await finishSetup();
      await waitForSettingsEssentials({});
      await navigate({ to: '/vpn-overview', replace: true });
      setTimeout(() => {
        useSetupWizardStore.getState().reset();
      }, 100);
    } catch (error) {
      console.error('Failed to finish setup flow:', error);
      useSetupWizardStore.setState({ isFinishing: false });
      Snackbar.error(m.initial_setup_confirmation_error_finish_failed());
    } finally {
      setIsSubmitting(false);
    }
  };

  const { mutateAsync: finishSetup } = useMutation({
    mutationKey: ['finish-setup'],
    mutationFn: api.initial_setup.finishSetup,
    onError: (error) => {
      console.error('Failed to finish setup:', error);
      Snackbar.error(m.initial_setup_confirmation_error_finish_failed());
    },
    meta: {
      invalidate: ['settings_essentials'],
    },
  });

  return (
    <WizardCard>
      <div className="confirmation">
        <div className="header">
          <h4>{m.initial_setup_confirmation_header()}</h4>
          <SizedBox height={ThemeSpacing.Sm} />
          <p>{m.initial_setup_confirmation_lead()}</p>
        </div>
        <Divider spacing={ThemeSpacing.Xl2} />
        <div className="content">
          <p className="title">{m.initial_setup_confirmation_title()}</p>
          <SizedBox height={ThemeSpacing.Md} />
          <ActionCard
            title={m.initial_setup_confirmation_action_title()}
            subtitle={m.initial_setup_confirmation_action_subtitle()}
            imageSrc={location}
          >
            <Icon icon={'transactions'} />
            <p>{m.initial_setup_confirmation_action_time()}</p>
          </ActionCard>
          <SizedBox height={ThemeSpacing.Xl2} />
          <p className="subtitle">{m.initial_setup_confirmation_footer()}</p>
        </div>
        <Controls>
          <Button
            text={m.initial_setup_confirmation_cancel()}
            onClick={handleExit}
            variant="outlined"
            disabled={isSubmitting}
          />
          <div className="right">
            <Button
              text={m.initial_setup_confirmation_submit()}
              onClick={handleFinish}
              loading={isSubmitting}
            />
          </div>
        </Controls>
      </div>
    </WizardCard>
  );
};
