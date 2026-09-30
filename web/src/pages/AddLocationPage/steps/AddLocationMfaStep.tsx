import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import { Card } from '../../../shared/components/Card/Card';
import { Controls } from '../../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { renderMfaFlowSelectionItem } from '../../../shared/components/MfaFlowSelectionItem/MfaFlowSelectionItem';
import type { SelectionOption } from '../../../shared/components/SelectionSection/type';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { FieldError } from '../../../shared/defguard-ui/components/FieldError/FieldError';
import { Fold } from '../../../shared/defguard-ui/components/Fold/Fold';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { Input } from '../../../shared/defguard-ui/components/Input/Input';
import { Radio } from '../../../shared/defguard-ui/components/Radio/Radio';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { getMfaFlowsQueryOptions } from '../../../shared/query';
import { AddLocationPageStep, type AddLocationPageStepValue } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';
import './style.scss';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';

const disconnectThresholdSchema = z
  .number(m.form_error_required())
  .min(120, m.form_error_min({ value: 120 }));

export const AddLocationMfaStep = () => {
  const { data: mfaFlows, isSuccess: mfaFlowsLoaded } = useQuery(getMfaFlowsQueryOptions);
  const hasMfaFlows = isPresent(mfaFlows) && mfaFlows.length > 0;
  const [mfaEnabledState, setMfaEnabledState] = useState(
    useAddLocationStore.getState().mfa_enabled,
  );
  const mfaEnabled = hasMfaFlows && mfaEnabledState;

  const [selectedFlowId, setSelectedFlowId] = useState<number | undefined>(
    useAddLocationStore.getState().mfa_flows.find((flow) => flow.is_default)?.flow_id,
  );
  const [disconnectThreshold, setDisconnectThreshold] = useState<number | null>(
    useAddLocationStore.getState().peer_disconnect_threshold,
  );
  const [thresholdError, setThresholdError] = useState<string | null>(null);
  const [continueAttempted, setContinueAttempted] = useState(false);

  useEffect(() => {
    if (mfaFlowsLoaded && !hasMfaFlows) {
      setMfaEnabledState(false);
      setSelectedFlowId(undefined);
    }
  }, [hasMfaFlows, mfaFlowsLoaded]);

  useEffect(() => {
    if (!mfaEnabled) {
      setThresholdError(null);
      return;
    }
    const result = disconnectThresholdSchema.safeParse(disconnectThreshold);
    setThresholdError(result.success ? null : (result.error.issues[0]?.message ?? null));
  }, [disconnectThreshold, mfaEnabled]);

  const saveAndContinue = (activeStep: AddLocationPageStepValue) => {
    useAddLocationStore.setState({
      mfa_enabled: mfaEnabled,
      mfa_flows:
        mfaEnabled && selectedFlowId !== undefined
          ? [{ flow_id: selectedFlowId, is_default: true, group_ids: [] }]
          : [],
      peer_disconnect_threshold: disconnectThreshold ?? 300,
      activeStep,
    });
  };

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
          setMfaEnabledState(false);
          setContinueAttempted(false);
        }}
        text={m.add_location_mfa_disable()}
        disabled={!hasMfaFlows}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <Radio
        active={mfaEnabled}
        onClick={() => {
          setMfaEnabledState(true);
          setContinueAttempted(false);
        }}
        text={m.add_location_mfa_assign_flow()}
        disabled={!hasMfaFlows}
      />
      <Fold open={mfaEnabled}>
        {hasMfaFlows && (
          <>
            <SizedBox height={ThemeSpacing.Xl2} />
            <Card className="add-location-mfa-flow-list">
              {mfaFlows?.map((flow, index) => {
                const option: SelectionOption<number, { steps: typeof flow.steps }> = {
                  id: flow.id,
                  label: flow.title,
                  meta: { steps: flow.steps },
                };
                return (
                  <div className="add-location-mfa-flow-row" key={flow.id}>
                    {index > 0 && <Divider />}
                    {renderMfaFlowSelectionItem({
                      option,
                      active: selectedFlowId === flow.id,
                      onClick: () => {
                        setSelectedFlowId(flow.id);
                        setContinueAttempted(false);
                      },
                    })}
                  </div>
                );
              })}
            </Card>
          </>
        )}
        <FieldError
          error={
            continueAttempted && mfaEnabled && selectedFlowId === undefined
              ? m.add_location_mfa_flow_required()
              : null
          }
        />
        {mfaEnabled && (
          <>
            <SizedBox height={ThemeSpacing.Xl2} />
            <Input
              label={m.location_mfa_label_client_disconnect_threshold()}
              helper={m.location_mfa_helper_client_disconnect_threshold()}
              type="number"
              value={disconnectThreshold}
              onChange={(value) => setDisconnectThreshold(value as number | null)}
              error={thresholdError}
              required
            />
          </>
        )}
      </Fold>
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => saveAndContinue(AddLocationPageStep.NetworkSettings)}
        />
        <div className="right">
          <Button
            text={m.controls_continue()}
            testId="finish"
            disabled={isPresent(thresholdError)}
            onClick={() => {
              if (mfaEnabled && selectedFlowId === undefined) {
                setContinueAttempted(true);
                return;
              }
              saveAndContinue(AddLocationPageStep.AccessControl);
            }}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
