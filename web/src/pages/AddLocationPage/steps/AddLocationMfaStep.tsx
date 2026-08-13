import { useEffect, useState } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Input } from '../../../shared/defguard-ui/components/Input/Input';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../../shared/defguard-ui/components/Toggle/Toggle';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

const schema = z
  .number(m.form_error_required())
  .min(120, m.form_error_min({ value: 120 }));

export const AddLocationMfaStep = () => {
  const [error, setError] = useState<string | null>(null);
  const [disconnect, setDisconnect] = useState<number | null>(300);
  const [mfaEnabled, setMfaEnabled] = useState(false);

  const handleSubmit = () => {
    if (!error) {
      useAddLocationStore.setState({
        mfa_enabled: mfaEnabled,
        activeStep: AddLocationPageStep.AccessControl,
      });
    }
  };

  useEffect(() => {
    if (!mfaEnabled) {
      setError(null);
      setDisconnect(300);
      return;
    }
    const result = schema.safeParse(disconnect);
    if (!result.success) {
      setError(result.error.issues[0]?.message ?? null);
    } else {
      setError(null);
    }
  }, [disconnect, mfaEnabled]);

  return (
    <WizardCard>
      <Toggle
        active={mfaEnabled}
        onClick={() => setMfaEnabled(!mfaEnabled)}
        label={m.add_location_mfa_toggle_label()}
        testId="toggle-mfa"
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      {mfaEnabled && (
        <Input
          label={m.location_mfa_label_client_disconnect_threshold()}
          helper={m.location_mfa_helper_client_disconnect_threshold()}
          type="number"
          value={disconnect}
          onChange={(value) => setDisconnect(value as number | null)}
          error={error}
          required
        />
      )}
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.NetworkSettings,
              peer_disconnect_threshold: disconnect ?? 300,
              mfa_enabled: mfaEnabled,
            });
          }}
        />
        <div className="right">
          <Button
            text={m.controls_continue()}
            testId="finish"
            disabled={isPresent(error)}
            onClick={() => {
              handleSubmit();
            }}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
