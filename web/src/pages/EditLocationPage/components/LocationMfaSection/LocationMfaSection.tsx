import './style.scss';

import { Link } from '@tanstack/react-router';
import { Reorder } from 'motion/react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import type {
  MfaFlowAssignment,
  MfaFlowListItemResponse,
} from '../../../../shared/api/types';
import { enterpriseBadgeProps } from '../../../../shared/components/badges/EnterpriseBadge';
import type { SelectionOption } from '../../../../shared/components/SelectionSection/type';
import { externalLink } from '../../../../shared/constants';
import { Badge } from '../../../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { IconKind } from '../../../../shared/defguard-ui/components/Icon';
import { Icon } from '../../../../shared/defguard-ui/components/Icon/Icon';
import { ThemeSpacing, ThemeVariable } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { mfaFlowUnavailableText } from '../../../../shared/utils/mfaFlowSteps';
import postureCheckShield from '../../assets/posture_check_shield.png';
import {
  overrideKey,
  removeOverride,
  reorderOverrides,
  selectableFlows,
  setDefaultFlow,
  splitAssignments,
  upsertOverride,
} from './assignments';
import { MfaFlowAssignmentCard } from './components/MfaFlowAssignmentCard/MfaFlowAssignmentCard';
import { MfaFlowAssignmentModal } from './components/MfaFlowAssignmentModal/MfaFlowAssignmentModal';
import { MfaFlowOverrideRow } from './components/MfaFlowOverrideRow/MfaFlowOverrideRow';

type Props = {
  assignments: MfaFlowAssignment[];
  flows: MfaFlowListItemResponse[];
  groupOptions: SelectionOption<number>[];
  canUseEnterprise: boolean;
  onChange: (assignments: MfaFlowAssignment[]) => void;
  /** Opens the default flow picker on mount when the location has no default. */
  openDefaultPicker?: boolean;
  /** Called when the default flow picker closes while the location still has no default. */
  onDefaultPickerDismissed?: () => void;
};

/** An override editor without an `index` creates a new assignment. */
type OpenEditor = { kind: 'default' } | { kind: 'override'; index?: number };

export const LocationMfaSection = ({
  assignments,
  flows,
  groupOptions,
  canUseEnterprise,
  onChange,
  openDefaultPicker,
  onDefaultPickerDismissed,
}: Props) => {
  const [editor, setEditor] = useState<OpenEditor>();
  const [modalOpen, setModalOpen] = useState(false);
  const overridesTrackRef = useRef<HTMLUListElement>(null);
  // The modal calls `onSubmit` and then `onClose` in one handler, before the new assignments
  // reach this component, so a choice made in the picker is tracked here.
  const defaultPickedRef = useRef(false);

  const openEditor = (next: OpenEditor) => {
    setEditor(next);
    setModalOpen(true);
  };

  const flowsById = useMemo(() => new Map(flows.map((flow) => [flow.id, flow])), [flows]);
  const groupNameById = useMemo(
    () => new Map(groupOptions.map((option) => [option.id, option.label])),
    [groupOptions],
  );

  const { defaultAssignment, overrides } = splitAssignments(assignments);

  const closeEditor = () => {
    const dismissedWithoutDefault =
      editor?.kind === 'default' &&
      !isPresent(defaultAssignment) &&
      !defaultPickedRef.current;
    defaultPickedRef.current = false;
    setModalOpen(false);
    if (dismissedWithoutDefault) {
      onDefaultPickerDismissed?.();
    }
  };

  // Opened from an effect because the modal must mount closed: ModalFoundation never fires
  // `afterClose` for a modal that was open on its first render. Keyed on `hasFlows` so the picker
  // also opens when the catalog goes from empty to non-empty while MFA is on.
  const hasFlows = flows.length > 0;
  // biome-ignore lint/correctness/useExhaustiveDependencies: only run when flows become available
  useEffect(() => {
    if (openDefaultPicker && hasFlows && !isPresent(defaultAssignment)) {
      openEditor({ kind: 'default' });
    }
  }, [hasFlows]);

  const overridesLocked = !canUseEnterprise;
  const hasUnavailableFlow = assignments.some((assignment) =>
    isPresent(flowsById.get(assignment.flow_id)?.unavailable_reason),
  );

  const editorTarget = useMemo(() => {
    if (editor === undefined) return undefined;
    switch (editor.kind) {
      case 'default':
        return { flowId: defaultAssignment?.flow_id, groupIds: undefined };
      case 'override': {
        const override = editor.index === undefined ? undefined : overrides[editor.index];
        return { flowId: override?.flow_id, groupIds: override?.group_ids ?? [] };
      }
    }
  }, [editor, defaultAssignment, overrides]);

  const editorFlows = useMemo(
    () => selectableFlows(flows, assignments, editorTarget?.flowId),
    [flows, assignments, editorTarget],
  );
  const allFlowsAssigned = selectableFlows(flows, assignments, undefined).length === 0;

  if (flows.length === 0) {
    return (
      <div className="location-empty-state">
        <img src={postureCheckShield} alt="" className="shield" />
        <p>
          {m.location_mfa_empty_state_before_link()}{' '}
          <Link to="/mfa/add-flow">{m.location_mfa_empty_state_create_link()}</Link>{' '}
          {m.location_mfa_empty_state_between_links()}{' '}
          <a href={externalLink.defguard.docs} target="_blank" rel="noreferrer">
            {m.location_mfa_empty_state_docs_link()}
          </a>{' '}
          {m.location_mfa_empty_state_after_link()}
        </p>
      </div>
    );
  }

  const handleSubmit = (flowId: number, groupIds: number[]) => {
    if (editor === undefined) return;
    switch (editor.kind) {
      case 'default':
        defaultPickedRef.current = true;
        onChange(setDefaultFlow(assignments, flowId));
        break;
      case 'override':
        onChange(upsertOverride(assignments, editor.index, flowId, groupIds));
        break;
    }
  };

  return (
    <>
      {isPresent(defaultAssignment) && (
        <>
          <div className="location-mfa-assignments">
            <Reorder.Group
              ref={overridesTrackRef}
              axis="y"
              values={overrides}
              onReorder={(next) => onChange(reorderOverrides(assignments, next))}
              className="overrides-track"
            >
              {overrides.map((override, index) => (
                <MfaFlowOverrideRow
                  key={overrideKey(override)}
                  override={override}
                  position={index + 1}
                  title={flowsById.get(override.flow_id)?.title ?? ''}
                  steps={flowsById.get(override.flow_id)?.steps ?? []}
                  chips={override.group_ids.map(
                    (id) => groupNameById.get(id) ?? String(id),
                  )}
                  unavailableText={mfaFlowUnavailableText(
                    flowsById.get(override.flow_id)?.unavailable_reason ?? null,
                  )}
                  dragConstraints={overridesTrackRef}
                  onEdit={() => openEditor({ kind: 'override', index })}
                  onRemove={() => onChange(removeOverride(assignments, index))}
                />
              ))}
            </Reorder.Group>
            <div className="assignment-row default-flow">
              {overrides.length > 0 && (
                <span className="marker fallback" aria-hidden="true">
                  →
                </span>
              )}
              <div className="default-flow-content">
                <MfaFlowAssignmentCard
                  title={flowsById.get(defaultAssignment.flow_id)?.title ?? ''}
                  steps={flowsById.get(defaultAssignment.flow_id)?.steps ?? []}
                  chips={[
                    overrides.length > 0
                      ? m.location_mfa_default_chip_everyone_else()
                      : m.location_mfa_default_chip_all_groups(),
                  ]}
                  unavailableText={mfaFlowUnavailableText(
                    flowsById.get(defaultAssignment.flow_id)?.unavailable_reason ?? null,
                  )}
                  leading={
                    <Icon
                      icon={IconKind.Groups}
                      size={20}
                      staticColor={ThemeVariable.FgAction}
                    />
                  }
                  editLabel={m.location_mfa_default_flow_edit()}
                  onEdit={() => openEditor({ kind: 'default' })}
                />
                {hasUnavailableFlow && (
                  <div className="location-mfa-error">
                    <div className="icon-track">
                      <Icon
                        icon={IconKind.WarningFilled}
                        size={20}
                        staticColor={ThemeVariable.FgCritical}
                      />
                    </div>
                    <p>{m.location_mfa_flows_unavailable_error()}</p>
                  </div>
                )}
              </div>
            </div>
          </div>
          <Divider spacing={ThemeSpacing.Xl} />
          <div className="location-mfa-overrides-header">
            <p className="title">{m.location_mfa_overrides_title()}</p>
            {overridesLocked && (
              <Badge
                {...enterpriseBadgeProps}
                text={m.license_enterprise_feature_badge()}
              />
            )}
          </div>
          <p className="location-mfa-overrides-description">
            {m.location_mfa_overrides_description()}
          </p>
          <Button
            variant="outlined"
            text={m.location_mfa_overrides_add()}
            disabled={overridesLocked || allFlowsAssigned}
            onClick={() => openEditor({ kind: 'override' })}
          />
        </>
      )}
      <MfaFlowAssignmentModal
        isOpen={modalOpen}
        target={editorTarget}
        flows={editorFlows}
        groupOptions={editor?.kind === 'override' ? groupOptions : undefined}
        onClose={closeEditor}
        afterClose={() => setEditor(undefined)}
        onSubmit={handleSubmit}
      />
    </>
  );
};
