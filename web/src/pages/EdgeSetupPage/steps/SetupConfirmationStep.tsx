import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { m } from '../../../paraglide/messages';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import addMoreImage from '../assets/add_more.svg';
import { useEdgeWizardStore } from '../useEdgeWizardStore';

export const SetupConfirmationStep = () => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const handleBack = () => {
    useEdgeWizardStore.getState().reset();
  };

  const handleFinish = () => {
    queryClient.invalidateQueries({ queryKey: ['edge'] });
    navigate({ to: '/edges', replace: true }).then(() => {
      setTimeout(() => {
        useEdgeWizardStore.getState().reset();
      }, 100);
    });
  };

  return (
    <WizardCard>
      <h2>{m.edge_setup_confirmation_title()}</h2>
      <SizedBox height={ThemeSpacing.Sm} />
      <p>{m.edge_setup_confirmation_subtitle()}</p>
      <Divider spacing={ThemeSpacing.Xl2} />
      <ActionCard
        title={m.edge_setup_add_multiple_edge_components_title()}
        subtitle={m.edge_setup_add_multiple_edge_components_subtitle()}
        imageSrc={addMoreImage}
      />
      <ModalControls
        cancelProps={{
          text: m.edge_setup_controls_add_another_edge_component(),
          onClick: handleBack,
          variant: 'outlined',
        }}
        submitProps={{
          text: m.edge_setup_controls_go_to_edge_components(),
          onClick: handleFinish,
        }}
      />
    </WizardCard>
  );
};
