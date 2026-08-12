import { useMutation } from '@tanstack/react-query';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../../../shared/defguard-ui/components/Toggle/Toggle';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { AutoAdoptionSetupStep } from '../types';
import { useAutoAdoptionSetupWizardStore } from '../useAutoAdoptionSetupWizardStore';

export const AutoAdoptionMfaSetupStep = () => {
  const setActiveStep = useAutoAdoptionSetupWizardStore((s) => s.setActiveStep);
  const mfaEnabled = useAutoAdoptionSetupWizardStore((s) => s.mfa_enabled);

  const { mutate: setMfaSettings, isPending } = useMutation({
    mutationFn: api.initial_setup.setAutoAdoptionMfaSettings,
    onSuccess: () => {
      setActiveStep(AutoAdoptionSetupStep.Summary);
    },
  });

  return (
    <WizardCard>
      <Toggle
        active={mfaEnabled}
        onClick={() =>
          useAutoAdoptionSetupWizardStore.setState({ mfa_enabled: !mfaEnabled })
        }
        label={m.add_location_mfa_toggle_label()}
        testId="toggle-mfa"
      />
      <SizedBox height={ThemeSpacing.Xl2} />
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
              setMfaSettings({ mfa_enabled: mfaEnabled });
            }}
            loading={isPending}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
