import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Icon } from '../../../shared/defguard-ui/components/Icon';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import location from '../assets/location.png';
import { useSetupWizardStore } from '../useSetupWizardStore';

export const SetupConfirmationStep = () => {
  const navigate = useNavigate();

  const { mutateAsync: finishSetup } = useMutation({
    mutationKey: ['finish-setup'],
    mutationFn: api.initial_setup.finishSetup,
    onError: (error) => {
      console.error('Failed to finish setup:', error);
      Snackbar.error(m.initial_setup_confirmation_error_finish_failed());
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
      </div>
      <ModalControls
        // Temporarily disabled
        // cancelProps={{ text: "I'll do this later", onClick: handleBack }}
        submitProps={{
          text: m.initial_setup_controls_finish(),
          onClick: handleFinish,
        }}
      />
    </WizardCard>
  );
};
