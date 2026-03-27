import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import { LocationMfaMode, type NetworkLocation } from '../../../shared/api/types';
import { businessBadgeProps } from '../../../shared/components/badges/BusinessBadge';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Input } from '../../../shared/defguard-ui/components/Input/Input';
import { InteractiveBlock } from '../../../shared/defguard-ui/components/InteractiveBlock/InteractiveBlock';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getLicenseInfoQueryOptions } from '../../../shared/query';
import { canUseBusinessFeature } from '../../../shared/utils/license';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

const schema = z
  .number(m.form_error_required())
  .min(120, m.form_min_value({ value: 120 }));

export const AddLocationMfaStep = () => {
  const [error, setError] = useState<string | null>(null);
  const [disconnect, setDisconnect] = useState<number | null>(300);
  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);
  const canUseFeature = useMemo(() => {
    if (licenseInfo === undefined) return undefined;
    return canUseBusinessFeature(licenseInfo).result;
  }, [licenseInfo]);

  const [choice, setChoice] = useState<NetworkLocation['location_mfa_mode']>(
    LocationMfaMode.Disabled,
  );

  const handleSubmit = () => {
    if (!error) {
      useAddLocationStore.setState({
        location_mfa_mode: choice,
        activeStep: AddLocationPageStep.AccessControl,
      });
    }
  };

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
      <InteractiveBlock
        value={choice === LocationMfaMode.Disabled}
        onClick={() => setChoice(LocationMfaMode.Disabled)}
        title={m.add_location_mfa_disabled_title()}
        data-testid="do-not-enforce-mfa"
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <InteractiveBlock
        value={choice === LocationMfaMode.Internal}
        onClick={() => setChoice(LocationMfaMode.Internal)}
        title={m.add_location_mfa_internal_title()}
        content={m.add_location_mfa_internal_content()}
        data-testid="enforce-internal-mfa"
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <InteractiveBlock
        value={choice === LocationMfaMode.External}
        onClick={() => setChoice(LocationMfaMode.External)}
        title={m.add_location_mfa_external_title()}
        content={m.add_location_mfa_external_content()}
        disabled={isPresent(canUseFeature) && !canUseFeature}
        badge={
          isPresent(canUseFeature) && !canUseFeature ? businessBadgeProps : undefined
        }
        data-testid="enforce-external-mfa"
      />
      {choice !== LocationMfaMode.Disabled && (
        <>
          <SizedBox height={ThemeSpacing.Xl2} />
          <Input
            label={m.location_mfa_label_client_disconnect_threshold()}
            type="number"
            value={disconnect}
            onChange={(value) => setDisconnect(value as number | null)}
            error={error}
            required
          />
        </>
      )}
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.NetworkSettings,
              peer_disconnect_threshold: disconnect ?? 300,
              location_mfa_mode: choice,
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
