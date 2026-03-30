import { useMutation } from '@tanstack/react-query';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { LocationMfaMode } from '../../../../shared/api/types';
import { businessBadgeProps } from '../../../../shared/components/badges/BusinessBadge';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { InfoBanner } from '../../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { InteractiveBlock } from '../../../../shared/defguard-ui/components/InteractiveBlock/InteractiveBlock';
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
      <InteractiveBlock
        value={mfaMode === LocationMfaMode.Disabled}
        onClick={() => setMfaMode(LocationMfaMode.Disabled)}
        title={m.initial_setup_auto_adoption_mfa_option_disabled_title()}
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <InteractiveBlock
        value={mfaMode === LocationMfaMode.Internal}
        onClick={() => setMfaMode(LocationMfaMode.Internal)}
        title={m.initial_setup_auto_adoption_mfa_option_internal_title()}
        content={m.initial_setup_auto_adoption_mfa_option_internal_content()}
      >
        {mfaMode === LocationMfaMode.Internal && (
          <>
            <SizedBox height={ThemeSpacing.Sm} />
            <InfoBanner
              variant="warning"
              icon="info-outlined"
              text={m.initial_setup_auto_adoption_mfa_option_internal_warning()}
            />
          </>
        )}
      </InteractiveBlock>
      <SizedBox height={ThemeSpacing.Xl} />
      <InteractiveBlock
        value={false}
        disabled
        title={m.initial_setup_auto_adoption_mfa_option_external_title()}
        content={m.initial_setup_auto_adoption_mfa_option_external_content()}
        badge={businessBadgeProps}
      ></InteractiveBlock>
      <Controls>
        <Button
          text={m.initial_setup_controls_back()}
          variant="outlined"
          onClick={() => setActiveStep(AutoAdoptionSetupStep.VpnSettings)}
        />
        <div className="right">
          <Button
            text={m.initial_setup_controls_continue()}
            onClick={() => {
              setMfaSettings({ vpn_mfa_mode: mfaMode });
            }}
            loading={isPending}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
