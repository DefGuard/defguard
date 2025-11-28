import { useNavigate } from '@tanstack/react-router';
import type { WizardPageStep } from '../../shared/components/wizard/types';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { AddLocationAccessStep } from './steps/AddLocationAccessStep';
import { AddLocationFirewallStep } from './steps/AddLocationFirewallStep';
import { AddLocationMfaStep } from './steps/AddLocationMfaStep';
import { AddLocationNetworkStep } from './steps/AddLocationNetworkStep';
import { AddLocationStartStep } from './steps/AddLocationStartStep/AddLocationStartStep';
import { AddLocationPageStep } from './types';
import { useAddLocationStore } from './useAddLocationStore';

const stepsData: WizardPageStep[] = [
  {
    id: AddLocationPageStep.Start,
    label: 'Public Facing Data',
    description: 'Manage core details and connection parameters for your VPN location.',
  },
  {
    id: AddLocationPageStep.VpnNetwork,
    label: 'Internal VPN & Network',
    description: 'Manage core details and connection parameters for your VPN location.',
  },
  {
    id: AddLocationPageStep.LocationAccess,
    label: 'Location Access',
    description: 'Assign user groups with access permissions to this location.',
  },
  {
    id: AddLocationPageStep.Firewall,
    label: 'Firewall',
    description:
      'The default policy defines how to handle traffic not covered by ACL rules.',
  },
  {
    id: AddLocationPageStep.Mfa,
    label: 'Multi-Factor Authentication',
    description:
      'Configure multi-factor authentication (MFA) to add a secondary verification step to user authentication.',
  },
];

const stepsComponents = [
  <AddLocationStartStep key={0} />,
  <AddLocationNetworkStep key={1} />,
  <AddLocationAccessStep key={2} />,
  <AddLocationFirewallStep key={3} />,
  <AddLocationMfaStep key={4} />,
];

export const AddLocationPage = () => {
  const navigate = useNavigate();
  const activeStep = useAddLocationStore((s) => s.activeStep);
  return (
    <WizardPage
      activeStep={activeStep}
      onClose={() => {
        navigate({
          to: '/locations',
        }).then(() => {
          setTimeout(() => {
            useAddLocationStore.getState().reset();
          }, 100);
        });
      }}
      subtitle="Welcome! Let's set up a new location to organize users, manage access, and connect gateways for activity tracking and monitoring."
      title="Create new location"
      steps={stepsData}
      id="add-location-wizard"
    >
      {stepsComponents[activeStep]}
    </WizardPage>
  );
};
