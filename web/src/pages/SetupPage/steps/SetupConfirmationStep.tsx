import { useNavigate } from '@tanstack/react-router';
import { m } from '../../../paraglide/messages';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Icon } from '../../../shared/defguard-ui/components/Icon';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import location from '../assets/location.png';
import { useSetupWizardStore } from '../useSetupWizardStore';

export const SetupConfirmationStep = () => {
  const navigate = useNavigate();

  const handleFinish = async () => {
    navigate({ to: '/add-location', replace: true }).then(() => {
      setTimeout(() => {
        useSetupWizardStore.getState().reset();
      }, 100);
    });
  };

  const handleExit = async () => {
    navigate({ to: '/vpn-overview', replace: true }).then(() => {
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
        <ModalControls
          cancelProps={{
            text: m.initial_setup_confirmation_cancel(),
            onClick: handleExit,
            variant: 'outlined',
          }}
          submitProps={{
            text: m.initial_setup_confirmation_submit(),
            onClick: handleFinish,
          }}
        />
      </div>
    </WizardCard>
  );
};
