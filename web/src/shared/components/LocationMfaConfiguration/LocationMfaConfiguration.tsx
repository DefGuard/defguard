import './style.scss';
import { Reorder, useDragControls } from 'motion/react';
import { sort } from 'radashi';
import { useCallback, useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { Divider } from '../../defguard-ui/components/Divider/Divider';
import { FieldError } from '../../defguard-ui/components/FieldError/FieldError';
import { Helper } from '../../defguard-ui/components/Helper/Helper';
import { Icon } from '../../defguard-ui/components/Icon';
import { ThemeSpacing, ThemeVariable } from '../../defguard-ui/types';
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

const availableOptions = Object.values(LocationMfaMethod) as LocationMfaMethodValue[];

export const LocationMfaConfiguration = ({
  onChange,
  steps,
  error,
}: LocationMfaConfigurationProps) => {
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
      next.delete(id);

      for (const [key, s] of next) {
        if (s.order > step.order) {
          next.set(key, { ...s, order: s.order - 1 });
        }
      }

      setInternalSteps(next);
      onChange(mapToSortedArray(next));
    },
    [internalSteps, onChange],
  );

  const onAddStep = useCallback(
    (id: string, initialFactor: LocationMfaMethodValue) => {
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
      next.set(stepId, { ...step, factors: step.factors.filter((f) => f !== factor) });

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
    () => availableOptions.filter((method) => !usedFactors.has(method)),
    [usedFactors],
  );

  const addStepMenuOptions = useMemo(
    () => [
      {
        items: availableMethods.map((key) => ({
          text: locationMfaMethodLabels[key],
          onClick: () => onAddStep(key, key),
        })),
      },
    ],
    [availableMethods, onAddStep],
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
              availableFactors={availableMethods}
              onDeleteStep={onDeleteStep}
              onAddFactor={onAddFactor}
              onDeleteFactor={onDeleteFactor}
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

const LocationMfaConfigurationStep = ({
  step,
  availableFactors,
  onDeleteStep,
  onAddFactor,
  onDeleteFactor,
}: {
  step: LocationMfaConfigurationStepData;
  availableFactors: LocationMfaMethodValue[];
  onDeleteStep: (id: string) => void;
  onAddFactor: (stepId: string, factor: LocationMfaMethodValue) => void;
  onDeleteFactor: (stepId: string, factor: LocationMfaMethodValue) => void;
}) => {
  const dragControls = useDragControls();

  const addFactorMenuOptions = useMemo(
    () => [
      {
        items: availableFactors.map((factor) => ({
          text: locationMfaMethodLabels[factor],
          onClick: () => onAddFactor(step.id, factor),
        })),
      },
    ],
    [availableFactors, onAddFactor, step.id],
  );

  return (
    <Reorder.Item
      value={step}
      dragListener={false}
      dragControls={dragControls}
      className="mfa-step-card"
      data-testid={`step-${step.id}`}
    >
      <div className="top">
        <button className="drag-button" onPointerDown={(e) => dragControls.start(e)}>
          <Icon icon="dnd" size={20} />
        </button>
        <p>{`Step ${step.order}`}</p>
        <button className="dispose-button" onClick={() => onDeleteStep(step.id)}>
          <Icon icon="delete" />
        </button>
      </div>
      <Divider spacing={ThemeSpacing.Lg} />
      <div className="factors">
        {step.factors.map((factor) => (
          <div key={factor} className="factor">
            <div className="track">
              <Icon icon="check-filled" staticColor={ThemeVariable.FgSuccess} />
              <p>{locationMfaMethodLabels[factor]}</p>
              <div className="right">
                <p>Mobile only</p>
                <button
                  className="dispose-button"
                  onClick={() => onDeleteFactor(step.id, factor)}
                >
                  <Icon icon="close" />
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
      {availableFactors.length > 0 && (
        <div className="footer">
          <LocationMfaMethodsMenu
            kind="plain"
            label="+ Add factor"
            options={addFactorMenuOptions}
          />
          <Helper>
            <p>{m.test_placeholder_long()}</p>
          </Helper>
        </div>
      )}
    </Reorder.Item>
  );
};
