import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Fold } from '../../../shared/defguard-ui/components/Fold/Fold';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { Input } from '../../../shared/defguard-ui/components/Input/Input';
import { Radio } from '../../../shared/defguard-ui/components/Radio/Radio';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getMfaFlowsQueryOptions } from '../../../shared/query';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';

const schema = z
  .number(m.form_error_required())
  .min(120, m.form_error_min({ value: 120 }));

export const AddLocationMfaStep = () => {
  const [error, setError] = useState<string | null>(null);
  const [disconnect, setDisconnect] = useState<number | null>(300);
  const [mfaEnabled, setMfaEnabled] = useState(false);
  const { data: mfaFlows } = useQuery(getMfaFlowsQueryOptions);
  const hasMfaFlows = mfaFlows?.length !== 0;

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
      <DescriptionBlock>
        <p>{m.add_location_step_mfa_flow_description()}</p>
      </DescriptionBlock>
      {!hasMfaFlows && (
        <>
          <SizedBox height={ThemeSpacing.Xl2} />
          <InfoBanner
            icon="warning-outlined"
            variant="warning"
            text={m.add_location_step_mfa_no_flows()}
          />
        </>
      )}
      <SizedBox height={ThemeSpacing.Xl} />
      <Radio
        active={!mfaEnabled}
        onClick={() => {
          setMfaEnabled(false);
        }}
        text={m.add_location_mfa_disable()}
        disabled={!hasMfaFlows}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <Radio
        active={mfaEnabled}
        onClick={() => {
          setMfaEnabled(true);
        }}
        text={m.add_location_mfa_assign_flow()}
        disabled={!hasMfaFlows}
      />
      <Fold open={mfaEnabled}>
        <SizedBox height={ThemeSpacing.Xl2} />
        <Input
          label={m.location_mfa_label_client_disconnect_threshold()}
          helper={m.location_mfa_helper_client_disconnect_threshold()}
          type="number"
          value={disconnect}
          onChange={(value) => setDisconnect(value as number | null)}
          error={error}
          required
        />
        <div>TODO</div>
      </Fold>
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
