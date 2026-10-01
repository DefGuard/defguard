import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { m } from '../../../paraglide/messages';
import { Card } from '../../../shared/components/Card/Card';
import { Controls } from '../../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import {
  type MfaFlowSelectionMeta,
  renderMfaFlowSelectionItem,
} from '../../../shared/components/MfaFlowSelectionItem/MfaFlowSelectionItem';
import type { SelectionOption } from '../../../shared/components/SelectionSection/type';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { FieldError } from '../../../shared/defguard-ui/components/FieldError/FieldError';
import { Fold } from '../../../shared/defguard-ui/components/Fold/Fold';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { Radio } from '../../../shared/defguard-ui/components/Radio/Radio';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import {
  getMfaFlowsQueryOptions,
  getMfaMethodAvailabilityQueryOptions,
} from '../../../shared/query';
import { AddLocationPageStep, type AddLocationPageStepValue } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

export const AddLocationMfaStep = () => {
  const {
    data: mfaFlows,
    isError: mfaFlowsFailed,
    isPending: mfaFlowsPending,
  } = useQuery(getMfaFlowsQueryOptions);
  const { data: methodAvailability } = useQuery(getMfaMethodAvailabilityQueryOptions);
  const hasMfaFlows = (mfaFlows?.length ?? 0) > 0;
  const noMfaFlows = !mfaFlowsPending && !mfaFlowsFailed && !hasMfaFlows;
  const storedMfaEnabled = useAddLocationStore((state) => state.mfa_enabled);
  const storedSelectedFlowId = useAddLocationStore(
    (state) => state.mfa_flows.find((flow) => flow.is_default)?.flow_id,
  );
  const [mfaEnabledState, setMfaEnabledState] = useState(storedMfaEnabled);
  const mfaEnabled = hasMfaFlows && mfaEnabledState;

  const [selectedFlowId, setSelectedFlowId] = useState<number | undefined>(
    storedSelectedFlowId,
  );
  const [flowError, setFlowError] = useState<string | undefined>();

  const saveAndContinue = (activeStep: AddLocationPageStepValue) => {
    useAddLocationStore.setState({
      mfa_enabled: mfaEnabled,
      mfa_flows:
        mfaEnabled && selectedFlowId !== undefined
          ? [{ flow_id: selectedFlowId, is_default: true, group_ids: [] }]
          : [],
      activeStep,
    });
  };

  return (
    <WizardCard>
      <DescriptionBlock>
        <p>{m.add_location_step_mfa_flow_description()}</p>
      </DescriptionBlock>
      {(noMfaFlows || mfaFlowsFailed) && (
        <>
          <SizedBox height={ThemeSpacing.Xl2} />
          <InfoBanner
            icon="warning-outlined"
            variant="warning"
            text={
              noMfaFlows
                ? m.add_location_step_mfa_no_flows()
                : m.add_location_step_mfa_flows_load_failed()
            }
          />
        </>
      )}
      <SizedBox height={ThemeSpacing.Xl} />
      <Radio
        active={!mfaEnabled}
        onClick={() => {
          setMfaEnabledState(false);
          setFlowError(undefined);
        }}
        text={m.add_location_mfa_disable()}
        disabled={mfaFlowsPending || mfaFlowsFailed || noMfaFlows}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <Radio
        active={mfaEnabled}
        onClick={() => {
          setMfaEnabledState(true);
          setFlowError(undefined);
        }}
        text={m.add_location_mfa_assign_flow()}
        disabled={mfaFlowsPending || mfaFlowsFailed || noMfaFlows}
      />
      <Fold open={mfaEnabled}>
        {hasMfaFlows && (
          <>
            <SizedBox height={ThemeSpacing.Xl2} />
            <Card className="add-location-mfa-flow-list">
              {mfaFlows?.map((flow, index) => {
                const option: SelectionOption<number, MfaFlowSelectionMeta> = {
                  id: flow.id,
                  label: flow.title,
                  meta: {
                    steps: flow.steps,
                    methodAvailability: methodAvailability?.methodAvailability,
                    unavailableReason: flow.unavailable_reason,
                  },
                };
                return (
                  <div className="add-location-mfa-flow-row" key={flow.id}>
                    {index > 0 && <Divider spacing={ThemeSpacing.Md} />}
                    {renderMfaFlowSelectionItem({
                      option,
                      active: selectedFlowId === flow.id,
                      onClick: () => {
                        setSelectedFlowId(flow.id);
                        setFlowError(undefined);
                      },
                    })}
                  </div>
                );
              })}
            </Card>
          </>
        )}
        <FieldError error={flowError} />
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
            disabled={mfaFlowsPending}
            onClick={() => {
              if (mfaEnabled && selectedFlowId === undefined) {
                setFlowError(m.add_location_mfa_flow_required());
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
