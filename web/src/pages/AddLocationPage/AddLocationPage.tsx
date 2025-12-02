import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import type { WizardPageStep } from '../../shared/components/wizard/types';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { AddLocationAccessStep } from './steps/AddLocationAccessStep';
import { AddLocationFirewallStep } from './steps/AddLocationFirewallStep';
import { AddLocationInternalVpnStep } from './steps/AddLocationInternalVpnStep';
import { AddLocationMfaStep } from './steps/AddLocationMfaStep';
import { AddLocationNetworkStep } from './steps/AddLocationNetworkStep';
import { AddLocationServiceStep } from './steps/AddLocationServiceStep';
import { AddLocationStartStep } from './steps/AddLocationStartStep';
import { AddLocationPageStep } from './types';
import { useAddLocationStore } from './useAddLocationStore';

export const AddLocationPage = () => {
  const navigate = useNavigate();
  const activeStep = useAddLocationStore((s) => s.activeStep);
  const locationType = useAddLocationStore((s) => s.locationType);

  const stepsData: WizardPageStep[] = useMemo(
    () => [
      {
        id: AddLocationPageStep.Start,
        label: 'Public Facing Data',
        description:
          'Manage core details and connection parameters for your VPN location.',
      },
      {
        id: AddLocationPageStep.InternalVpnSettings,
        label: 'Internal VPN ',
        description:
          'Manage core details and connection parameters for your VPN location.',
      },
      {
        id: AddLocationPageStep.NetworkSettings,
        label: 'Network Settings',
        description:
          'Manage core details and connection parameters for your VPN location.',
      },
      locationType === 'regular'
        ? {
            id: AddLocationPageStep.Mfa,
            label: 'Multi-Factor Authentication',
            description:
              'Configure multi-factor authentication (MFA) to add a secondary verification step to user authentication.',
          }
        : {
            id: AddLocationPageStep.ServiceLocationSettings,
            label: 'Service Location Settings',
            description:
              'A special kind of locations that allow establishing automatic VPN connections on system boot. Service locations are currently only supported with Defguard Client for Windows.',
          },
      {
        id: AddLocationPageStep.AccessControl,
        label: 'Access Control',
        description: 'Assign user groups with access permissions to this location.',
      },
      {
        id: AddLocationPageStep.Firewall,
        label: 'Firewall',
        description:
          'The default policy defines how to handle traffic not covered by ACL rules.',
      },
    ],
    [locationType],
  );

  const stepsComponents = useMemo(
    () => [
      <AddLocationStartStep key={0} />,
      <AddLocationInternalVpnStep key={1} />,
      <AddLocationNetworkStep key={2} />,
      locationType === 'regular' ? (
        <AddLocationMfaStep key={3} />
      ) : (
        <AddLocationServiceStep key={3} />
      ),
      <AddLocationAccessStep key={4} />,
      <AddLocationFirewallStep key={5} />,
    ],
    [locationType],
  );

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
