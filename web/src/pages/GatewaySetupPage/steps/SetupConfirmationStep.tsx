import { useNavigate } from '@tanstack/react-router';
import { m } from '../../../paraglide/messages';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import addMoreImage from '../assets/add_more.svg';
import { useGatewayWizardStore } from '../useGatewayWizardStore';

export const SetupConfirmationStep = () => {
  const navigate = useNavigate();

  const handleBack = () => {
    const networkId = useGatewayWizardStore.getState().network_id;
    useGatewayWizardStore.getState().reset();
    useGatewayWizardStore.getState().start({ network_id: networkId });
  };

  const handleFinish = () => {
    navigate({ to: '/locations', replace: true }).then(() => {
      setTimeout(() => {
        useGatewayWizardStore.getState().reset();
      }, 100);
    });
  };

  return (
    <WizardCard>
      <h2>{m.gateway_setup_confirmation_title()}</h2>
      <SizedBox height={ThemeSpacing.Sm} />
      <p>{m.gateway_setup_confirmation_subtitle()}</p>
      <Divider spacing={ThemeSpacing.Xl2} />
      <ActionCard
        title={m.gateway_setup_add_multiple_gateways_title()}
        subtitle={m.gateway_setup_add_multiple_gateways_subtitle()}
        imageSrc={addMoreImage}
      />
      <ModalControls
        cancelProps={{
          text: m.gateway_setup_controls_add_another_gateway(),
          onClick: handleBack,
          variant: 'outlined',
        }}
        submitProps={{
          text: m.gateway_setup_controls_go_to_locations(),
          onClick: handleFinish,
        }}
      />
    </WizardCard>
  );
};
