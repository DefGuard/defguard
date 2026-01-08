import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useMemo } from 'react';
import type { WizardPageStep } from '../../shared/components/wizard/types';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { AddLocationAccessStep } from './steps/AddLocationAccessStep';
import { AddLocationFirewallStep } from './steps/AddLocationFirewallStep';
import { AddLocationInternalVpnStep } from './steps/AddLocationInternalVpnStep';
import { AddLocationMfaStep } from './steps/AddLocationMfaStep';
import { AddLocationNetworkStep } from './steps/AddLocationNetworkStep';
import { AddLocationServiceStep } from './steps/AddLocationServiceStep';
import { AddLocationStartStep } from './steps/AddLocationStartStep';
import { AddLocationPageStep, type AddLocationPageStepValue } from './types';
import { useAddLocationStore } from './useAddLocationStore';

export const AddLocationPage = () => {
  const navigate = useNavigate();
  const activeStep = useAddLocationStore((s) => s.activeStep);
  const locationType = useAddLocationStore((s) => s.locationType);

  const stepsConfig = useMemo(
    (): Record<AddLocationPageStepValue, WizardPageStep> => ({
      start: {
        id: AddLocationPageStep.Start,
        order: 0,
        label: 'Public Facing Data',
        description:
          'Manage core details and connection parameters for your VPN location.',
      },
      internalVpnSettings: {
        id: AddLocationPageStep.InternalVpnSettings,
        order: 1,
        label: 'Internal VPN ',
        description:
          'Manage core details and connection parameters for your VPN location.',
      },
      networkSettings: {
        id: AddLocationPageStep.NetworkSettings,
        order: 2,
        label: 'Network Settings',
        description:
          'Manage core details and connection parameters for your VPN location.',
      },
      mfa: {
        id: AddLocationPageStep.Mfa,
        order: 3,
        label: 'Multi-Factor Authentication',
        hidden: locationType === 'service',
        description:
          'Configure multi-factor authentication (MFA) to add a secondary verification step to user authentication.',
      },
      serviceLocationSettings: {
        id: AddLocationPageStep.ServiceLocationSettings,
        order: 4,
        hidden: locationType === 'regular',
        label: 'Service Location Settings',
        description:
          'A special kind of locations that allow establishing automatic VPN connections on system boot. Service locations are currently only supported with Defguard Client for Windows.',
      },
      accessControl: {
        id: AddLocationPageStep.AccessControl,
        order: 5,
        label: 'Access Control',
        description: 'Assign user groups with access permissions to this location.',
      },
      firewall: {
        id: AddLocationPageStep.Firewall,
        order: 6,
        label: 'Firewall',
        description:
          'The default policy defines how to handle traffic not covered by ACL rules.',
      },
    }),
    [locationType],
  );

  const stepsComponents = useMemo(
    (): Record<AddLocationPageStepValue, ReactNode> => ({
      start: <AddLocationStartStep />,
      accessControl: <AddLocationAccessStep />,
      firewall: <AddLocationFirewallStep />,
      internalVpnSettings: <AddLocationInternalVpnStep />,
      mfa: <AddLocationMfaStep />,
      networkSettings: <AddLocationNetworkStep />,
      serviceLocationSettings: <AddLocationServiceStep />,
    }),
    [],
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
      steps={stepsConfig}
      id="add-location-wizard"
    >
      {stepsComponents[activeStep]}
    </WizardPage>
  );
};
