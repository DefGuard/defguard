import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import api from '../../../shared/api/api';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Icon } from '../../../shared/defguard-ui/components/Icon';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import location from '../assets/location.svg';
import { useSetupWizardStore } from '../useSetupWizardStore';

export const SetupConfirmationStep = () => {
  const navigate = useNavigate();

  const { mutateAsync: finishSetup } = useMutation({
    mutationKey: ['finish-setup'],
    mutationFn: api.initial_setup.finishSetup,
    onError: (error) => {
      console.error('Failed to finish setup:', error);
      Snackbar.error('Failed to finish setup. Please try again.');
    },
    meta: {
      invalidate: ['settings-essentials'],
    },
  });

  const handleFinish = async () => {
    await finishSetup();
    navigate({ to: '/auth/login', replace: true }).then(() => {
      setTimeout(() => {
        useSetupWizardStore.getState().reset();
      }, 100);
    });
  };

  return (
    <WizardCard>
      <div className="confirmation">
        <div className="header">
          <h4>General system settings are complete.</h4>
          <SizedBox height={ThemeSpacing.Sm} />
          <p>
            You've completed the first stage of the setup. Defguard is almost ready to go.
          </p>
        </div>
        <Divider spacing={ThemeSpacing.Xl2} />
        <div className="content">
          <p className="title">In order to fully deploy Defguard you need:</p>
          <SizedBox height={ThemeSpacing.Md} />
          <ActionCard
            title="Create first location."
            subtitle="To organize users, manage access, track users activity and device monitoring."
            imageSrc={location}
          >
            <Icon icon={'transactions'} />
            <p>Around 3 minutes</p>
          </ActionCard>
          <SizedBox height={ThemeSpacing.Xl2} />
          <p className="subtitle">
            Once you create your first location, the only step left will be to connect a
            gateway — and the system will be fully ready to use. This usually takes about
            n 10–15 minutes, depending on the complexity of your VPN configuration.
          </p>
        </div>
      </div>
      <ModalControls
        // Temporarily disabled
        // cancelProps={{ text: "I'll do this later", onClick: handleBack }}
        submitProps={{ text: 'Finish', onClick: handleFinish }}
      />
    </WizardCard>
  );
};
