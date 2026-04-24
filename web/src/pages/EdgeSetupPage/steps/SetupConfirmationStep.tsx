import { useNavigate } from '@tanstack/react-router';
import { m } from '../../../paraglide/messages';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { WizardSuccessHeader } from '../../../shared/components/wizard/WizardSuccessHeader/WizardSuccessHeader';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import addMoreImage from '../assets/add_more.svg';
import { useEdgeWizardStore } from '../useEdgeWizardStore';

export const SetupConfirmationStep = () => {
  const navigate = useNavigate();

  const handleBack = () => {
    useEdgeWizardStore.getState().reset();
  };

  const handleFinish = () => {
    navigate({ to: '/edges', replace: true }).then(() => {
      setTimeout(() => {
        useEdgeWizardStore.getState().reset();
      }, 100);
    });
  };

  return (
    <WizardCard>
      <WizardSuccessHeader title={m.edge_setup_confirmation_title()}>
        <p>{m.edge_setup_confirmation_subtitle()}</p>
      </WizardSuccessHeader>
      <Divider spacing={ThemeSpacing.Xl2} />
      <ActionCard
        title={m.edge_setup_add_multiple_edge_components_title()}
        subtitle={m.edge_setup_add_multiple_edge_components_subtitle()}
        imageSrc={addMoreImage}
      />
      <Controls>
        <div className="right">
          <Button
            text={m.edge_setup_controls_add_another_edge_component()}
            onClick={handleBack}
            variant="outlined"
          />
          <Button
            text={m.edge_setup_controls_go_to_edge_components()}
            onClick={handleFinish}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
