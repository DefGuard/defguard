import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useCallback, useMemo } from 'react';
import { m } from '../../paraglide/messages';
import type {
  WizardPageStep,
  WizardWelcomePageConfig,
} from '../../shared/components/wizard/types';
import { WizardCoverImage } from '../../shared/components/wizard/WizardCoverImage/WizardCoverImage';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { AddLocationAccessStep } from './steps/AddLocationAccessStep';
import { AddLocationFirewallStep } from './steps/AddLocationFirewallStep';
import { AddLocationInternalVpnStep } from './steps/AddLocationInternalVpnStep';
import { AddLocationMfaStep } from './steps/AddLocationMfaStep';
import { AddLocationNetworkStep } from './steps/AddLocationNetworkStep';
import { AddLocationServiceStep } from './steps/AddLocationServiceStep';
import { AddLocationStartStep } from './steps/AddLocationStartStep';
import { AddLocationWelcomeStep } from './steps/AddLocationWelcomeStep';
import { AddLocationPageStep, type AddLocationPageStepValue } from './types';
import { useAddLocationStore } from './useAddLocationStore';

export const AddLocationPage = () => {
  const navigate = useNavigate();
  const activeStep = useAddLocationStore((s) => s.activeStep);
  const locationType = useAddLocationStore((s) => s.locationType);
  const isWelcome = useAddLocationStore((s) => s.isWelcome);

  const onClose = useCallback(() => {
    navigate({
      to: '/locations',
    }).then(() => {
      setTimeout(() => {
        useAddLocationStore.getState().reset();
      }, 100);
    });
  }, [navigate]);

  const welcomeConfig = useMemo(
    (): WizardWelcomePageConfig => ({
      title: m.add_location_welcome_title(),
      subtitle: m.add_location_welcome_subtitle(),
      content: <AddLocationWelcomeStep />,
      displayDocs: false,
      media: <WizardCoverImage variant="location" />,
      onClose,
    }),
    [onClose],
  );

  const stepsConfig = useMemo(
    (): Record<AddLocationPageStepValue, WizardPageStep> => ({
      start: {
        id: AddLocationPageStep.Start,
        order: 0,
        label: m.add_location_step_public_facing_data_label(),
        description: m.add_location_step_common_description(),
      },
      internalVpnSettings: {
        id: AddLocationPageStep.InternalVpnSettings,
        order: 1,
        label: m.add_location_step_internal_vpn_label(),
        description: m.add_location_step_common_description(),
      },
      networkSettings: {
        id: AddLocationPageStep.NetworkSettings,
        order: 2,
        label: m.add_location_step_network_settings_label(),
        description: m.add_location_step_common_description(),
      },
      mfa: {
        id: AddLocationPageStep.Mfa,
        order: 3,
        label: m.add_location_step_mfa_label(),
        hidden: locationType === 'service',
        description: m.add_location_step_mfa_description(),
      },
      serviceLocationSettings: {
        id: AddLocationPageStep.ServiceLocationSettings,
        order: 4,
        hidden: locationType === 'regular',
        label: m.add_location_step_service_location_settings_label(),
        description: m.add_location_step_service_location_settings_description(),
      },
      accessControl: {
        id: AddLocationPageStep.AccessControl,
        order: 5,
        label: m.add_location_step_access_control_label(),
        description: m.add_location_step_access_control_description(),
      },
      firewall: {
        id: AddLocationPageStep.Firewall,
        order: 6,
        label: m.add_location_step_firewall_label(),
        description: m.add_location_step_firewall_description(),
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
      isOnWelcomePage={isWelcome}
      activeStep={activeStep}
      onClose={onClose}
      subtitle={m.add_location_welcome_subtitle()}
      title={m.add_location_page_title()}
      steps={stepsConfig}
      id="add-location-wizard"
      welcomePageConfig={welcomeConfig}
    >
      {stepsComponents[activeStep]}
    </WizardPage>
  );
};
