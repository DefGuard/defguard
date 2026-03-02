import { useMutation } from '@tanstack/react-query';
import api from '../../../../shared/api/api';
import { LocationMfaMode } from '../../../../shared/api/types';
import { businessBadgeProps } from '../../../../shared/components/badges/BusinessBadge';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { InfoBanner } from '../../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { InteractiveBlock } from '../../../../shared/defguard-ui/components/InteractiveBlock/InteractiveBlock';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { AutoAdoptionSetupStep } from '../types';
import { useAutoAdoptionSetupWizardStore } from '../useAutoAdoptionSetupWizardStore';

export const AutoAdoptionMfaSetupStep = () => {
  const setActiveStep = useAutoAdoptionSetupWizardStore((s) => s.setActiveStep);
  const mfaMode = useAutoAdoptionSetupWizardStore((s) => s.vpn_mfa_mode);

  const { mutate: setMfaSettings, isPending } = useMutation({
    mutationFn: api.initial_setup.setAutoAdoptionMfaSettings,
    onSuccess: () => {
      setActiveStep(AutoAdoptionSetupStep.Summary);
    },
  });

  const setMfaMode = (mode: (typeof LocationMfaMode)[keyof typeof LocationMfaMode]) => {
    useAutoAdoptionSetupWizardStore.setState({ vpn_mfa_mode: mode });
  };

  return (
    <WizardCard>
      <div className="auto-adoption-mfa-step">
        <InteractiveBlock
          value={mfaMode === LocationMfaMode.Disabled}
          onClick={() => setMfaMode(LocationMfaMode.Disabled)}
          title="Do not enforce MFA"
        />
        <SizedBox height={ThemeSpacing.Xl} />
        <InteractiveBlock
          value={mfaMode === LocationMfaMode.Internal}
          onClick={() => setMfaMode(LocationMfaMode.Internal)}
          title="Internal Defguard Multi-Factor Authentication"
          content="Uses the MFA methods configured in your Defguard profile."
        >
          {mfaMode === LocationMfaMode.Internal && (
            <>
              <SizedBox height={ThemeSpacing.Sm} />
              <InfoBanner
                variant="warning"
                icon="info-outlined"
                text="After completing the initial DefGuard setup, configure MFA in your profile to enable it."
              />
            </>
          )}
        </InteractiveBlock>
        <SizedBox height={ThemeSpacing.Xl} />
        <InteractiveBlock
          value={false}
          disabled
          title="External Identity Provider Authentication"
          content="Requires configuring an external identity provider in the settings, such as Google, Microsoft Entra ID, Okta, or JumpCloud."
          badge={businessBadgeProps}
        ></InteractiveBlock>
      </div>
      <ModalControls
        cancelProps={{
          text: 'Back',
          variant: 'outlined',
          onClick: () => setActiveStep(AutoAdoptionSetupStep.VpnSettings),
        }}
        submitProps={{
          text: 'Continue',
          onClick: () => {
            setMfaSettings({ vpn_mfa_mode: mfaMode });
          },
          loading: isPending,
        }}
      />
    </WizardCard>
  );
};
