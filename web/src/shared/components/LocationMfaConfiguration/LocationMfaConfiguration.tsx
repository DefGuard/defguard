import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { Reorder } from 'motion/react';
import { sort } from 'radashi';
import { useCallback, useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { FieldError } from '../../defguard-ui/components/FieldError/FieldError';
import { useApp } from '../../hooks/useApp';
import { getLicenseInfoQueryOptions } from '../../query';
import { canUseEnterpriseFeature } from '../../utils/license';
import { LocationMfaConfigurationStep } from './components/LocationMfaConfigurationStep';
import { LocationMfaMethodsMenu } from './components/LocationMfaMethodsMenu';
import type {
  LocationMfaConfigurationProps,
  LocationMfaConfigurationStepData,
  LocationMfaMethodValue,
} from './types';
import { LocationMfaMethod, locationMfaMethodLabels } from './types';

type InternalStepsMap = Map<string, LocationMfaConfigurationStepData>;

const mapToSortedArray = (map: InternalStepsMap): LocationMfaConfigurationStepData[] =>
  sort(Array.from(map.values()), (s) => s.order);

const deleteAndReorder = (
  map: InternalStepsMap,
  deleted: LocationMfaConfigurationStepData,
): void => {
  map.delete(deleted.id);
  for (const [key, s] of map) {
    if (s.order > deleted.order) map.set(key, { ...s, order: s.order - 1 });
  }
};

export const LocationMfaConfiguration = ({
  onChange,
  steps,
  error,
}: LocationMfaConfigurationProps) => {
  const smtpAvailable = useApp((s) => s.appInfo.smtp_enabled);

  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);

  const isEnterprise = useMemo(
    () => canUseEnterpriseFeature(licenseInfo ?? null),
    [licenseInfo],
  );

  const [internalSteps, setInternalSteps] = useState<InternalStepsMap>(
    () => new Map(steps.map((step) => [step.id, step])),
  );

  const onReorder = useCallback(
    (newSteps: LocationMfaConfigurationStepData[]) => {
      const reordered = newSteps.map((s, i) => ({ ...s, order: i + 1 }));
      const next: InternalStepsMap = new Map(reordered.map((s) => [s.id, s]));
      setInternalSteps(next);
      onChange(reordered);
    },
    [onChange],
  );

  const onDeleteStep = useCallback(
    (id: string) => {
      const step = internalSteps.get(id);
      if (!step) return;

      const next: InternalStepsMap = new Map(internalSteps);
      deleteAndReorder(next, step);

      setInternalSteps(next);
      onChange(mapToSortedArray(next));
    },
    [internalSteps, onChange],
  );

  const onAddStep = useCallback(
    (initialFactor: LocationMfaMethodValue) => {
      const id = crypto.randomUUID();
      const next: InternalStepsMap = new Map(internalSteps);
      next.set(id, { id, order: next.size + 1, factors: [initialFactor] });

      setInternalSteps(next);
      onChange(mapToSortedArray(next));
    },
    [internalSteps, onChange],
  );

  const onAddFactor = useCallback(
    (stepId: string, factor: LocationMfaMethodValue) => {
      const step = internalSteps.get(stepId);
      if (!step) return;

      const next: InternalStepsMap = new Map(internalSteps);
      next.set(stepId, { ...step, factors: [...step.factors, factor] });

      setInternalSteps(next);
      onChange(mapToSortedArray(next));
    },
    [internalSteps, onChange],
  );

  const onDeleteFactor = useCallback(
    (stepId: string, factor: LocationMfaMethodValue) => {
      const step = internalSteps.get(stepId);
      if (!step) return;

      const next: InternalStepsMap = new Map(internalSteps);
      const remaining = step.factors.filter((f) => f !== factor);
      if (remaining.length === 0) {
        deleteAndReorder(next, step);
      } else {
        next.set(stepId, { ...step, factors: remaining });
      }

      setInternalSteps(next);
      onChange(mapToSortedArray(next));
    },
    [internalSteps, onChange],
  );

  const sortedSteps = useMemo(() => mapToSortedArray(internalSteps), [internalSteps]);

  const usedFactors = useMemo(
    () => new Set(sortedSteps.flatMap((s) => s.factors)),
    [sortedSteps],
  );

  const availableMethods = useMemo(
    () =>
      [
        LocationMfaMethod.Email,
        LocationMfaMethod.MobileConfirm,
        LocationMfaMethod.Totp,
        LocationMfaMethod.Biometry,
        LocationMfaMethod.Fido2,
        LocationMfaMethod.Tpm,
        LocationMfaMethod.OpenId,
      ].filter((method) => !usedFactors.has(method)),
    [usedFactors],
  );

  const buildOption = useCallback(
    (method: LocationMfaMethodValue, onClick: () => void) => {
      const isEmailWithoutSmtp = method === LocationMfaMethod.Email && !smtpAvailable;
      let disabledHelper: string | undefined;
      let disabled = false;

      if (isEmailWithoutSmtp) {
        disabledHelper = m.cmp_location_mfa_smtp_disabled();
        disabled = true;
      }

      if (!isEnterprise) {
        if (method === LocationMfaMethod.Tpm || method === LocationMfaMethod.OpenId) {
          disabled = true;
          disabledHelper = m.cmp_location_mfa_enterprise_required();
        }
      }
      return {
        text: locationMfaMethodLabels[method],
        disabled,
        disabledHelper,
        onClick,
      };
    },
    [isEnterprise, smtpAvailable],
  );

  const methodGroups = useMemo(() => {
    if (isEnterprise) {
      return [{ header: undefined, items: availableMethods }];
    }

    const planMethods = [
      LocationMfaMethod.Email,
      LocationMfaMethod.MobileConfirm,
      LocationMfaMethod.Totp,
      LocationMfaMethod.Biometry,
      LocationMfaMethod.Fido2,
    ].filter((m) => availableMethods.includes(m));

    const higherPlanMethods = [LocationMfaMethod.Tpm, LocationMfaMethod.OpenId].filter(
      (m) => availableMethods.includes(m),
    );

    return [
      { header: { text: 'Available in your plan' }, items: planMethods },
      { header: { text: 'Available in higher plans' }, items: higherPlanMethods },
    ];
  }, [availableMethods, isEnterprise]);

  const addStepMenuOptions = useMemo(
    () =>
      methodGroups.map((group) => ({
        ...group,
        items: group.items.map((method) => buildOption(method, () => onAddStep(method))),
      })),
    [methodGroups, buildOption, onAddStep],
  );

  return (
    <div className="location-mfa-configuration">
      <div className="main-track">
        <Reorder.Group
          axis="y"
          values={sortedSteps}
          onReorder={onReorder}
          className="steps-track"
        >
          {sortedSteps.map((step) => (
            <LocationMfaConfigurationStep
              key={step.id}
              step={step}
              methodGroups={methodGroups}
              onDeleteStep={onDeleteStep}
              onAddFactor={onAddFactor}
              onDeleteFactor={onDeleteFactor}
              buildOption={buildOption}
            />
          ))}
        </Reorder.Group>
      </div>
      {availableMethods.length > 0 && (
        <div className="actions">
          <LocationMfaMethodsMenu
            kind="button"
            variant="outlined"
            iconRight="arrow-small"
            iconRightRotation="down"
            text="Add MFA step"
            options={addStepMenuOptions}
          />
        </div>
      )}
      <FieldError error={error} />
    </div>
  );
};
