import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { omit } from 'lodash-es';
import { useEffect, useState } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { LocationMfaMode, type NetworkLocation } from '../../../shared/api/types';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Input } from '../../../shared/defguard-ui/components/Input/Input';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { Radio } from '../../../shared/defguard-ui/components/Radio/Radio';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

const schema = z
  .number(m.form_error_required())
  .min(120, m.form_min_value({ value: 120 }));

export const AddLocationMfaStep = () => {
  const [error, setError] = useState<string | null>(null);
  const [disconnect, setDisconnect] = useState<number | null>(300);
  const navigate = useNavigate();

  const [choice, setChoice] = useState<NetworkLocation['location_mfa_mode']>(
    LocationMfaMode.Disabled,
  );

  const { mutate, isPending } = useMutation({
    mutationFn: api.location.addLocation,
    meta: {
      invalidate: ['network'],
    },
    onSuccess: () => {
      navigate({ to: '/locations', replace: true }).then(() => {
        setTimeout(() => {
          useAddLocationStore.getState().reset();
        }, 100);
      });
    },
  });

  useEffect(() => {
    if (choice === LocationMfaMode.Disabled) {
      setError(null);
      setDisconnect(300);
      return;
    }
    const result = schema.safeParse(disconnect);
    if (!result.success) {
    } else {
      setError(null);
    }
  }, [disconnect, choice]);

  return (
    <WizardCard>
      <Radio
        active={choice === LocationMfaMode.Disabled}
        onClick={() => setChoice(LocationMfaMode.Disabled)}
        text="Do not enforce MFA"
      />
      <SizedBox height={ThemeSpacing.Md} />
      <Radio
        active={choice === LocationMfaMode.Internal}
        onClick={() => setChoice(LocationMfaMode.Internal)}
        text="Internal MFA"
      />
      <SizedBox height={ThemeSpacing.Md} />
      <Radio
        active={choice === LocationMfaMode.External}
        onClick={() => setChoice(LocationMfaMode.External)}
        text="External MFA"
      />
      {choice !== LocationMfaMode.Disabled && (
        <>
          <SizedBox height={ThemeSpacing.Xl2} />
          <Input
            label="Client disconnect threshold (seconds)"
            type="number"
            value={disconnect}
            onChange={(value) => setDisconnect(value as number | null)}
            error={error}
            required
          />
        </>
      )}
      <ModalControls
        submitProps={{
          text: m.controls_finish(),
          disabled: isPresent(error),
          onClick: () => {
            const storageState = omit(useAddLocationStore.getState(), [
              'activeStep',
              'reset',
            ]);
            mutate({
              ...storageState,
              peer_disconnect_threshold: disconnect as number,
            });
          },
          loading: isPending,
        }}
      >
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.Firewall,
              peer_disconnect_threshold: disconnect ?? 300,
              location_mfa_mode: choice,
            });
          }}
        />
      </ModalControls>
    </WizardCard>
  );
};
