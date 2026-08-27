import './modalStyle.scss';

import { useMemo, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import type { MfaFlowListItemResponse } from '../../../../shared/api/types';
import { Controls } from '../../../../shared/components/Controls/Controls';
import {
  type MfaFlowSelectionMeta,
  renderMfaFlowSelectionItem,
} from '../../../../shared/components/MfaFlowSelectionItem/MfaFlowSelectionItem';
import { SelectionSection } from '../../../../shared/components/SelectionSection/SelectionSection';
import type { SelectionOption } from '../../../../shared/components/SelectionSection/type';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { FieldError } from '../../../../shared/defguard-ui/components/FieldError/FieldError';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';

type Props = {
  isOpen: boolean;
  flows: MfaFlowListItemResponse[];
  /** Provided when editing a group-scoped override. */
  groupOptions?: SelectionOption<number>[];
  initialFlowId?: number;
  initialGroupIds?: number[];
  onClose: () => void;
  onSubmit: (flowId: number, groupIds: number[]) => void;
};

export const MfaFlowAssignmentModal = ({
  isOpen,
  flows,
  groupOptions,
  initialFlowId,
  initialGroupIds,
  onClose,
  onSubmit,
}: Props) => {
  const editsGroups = isPresent(groupOptions);
  const [onGroupStep, setOnGroupStep] = useState(false);
  const [flowId, setFlowId] = useState(initialFlowId);
  const [groupIds, setGroupIds] = useState(new Set(initialGroupIds ?? []));
  const [groupError, setGroupError] = useState<string>();

  const flowSelectionOptions = useMemo(
    (): SelectionOption<number, MfaFlowSelectionMeta>[] =>
      flows.map((flow) => ({
        id: flow.id,
        label: flow.title,
        meta: { steps: flow.steps },
      })),
    [flows],
  );

  const title = useMemo(() => {
    if (!editsGroups) return m.location_mfa_default_flow_modal_title();
    return onGroupStep
      ? m.location_mfa_override_groups_modal_title()
      : m.location_mfa_override_flow_modal_title();
  }, [editsGroups, onGroupStep]);

  // Keep the selected flow when its radio item is clicked again.
  const handleFlowChange = (next: Set<number>) => {
    const picked = [...next].find((id) => id !== flowId);
    if (picked !== undefined) setFlowId(picked);
  };

  const handleGroupChange = (next: Set<number>) => {
    setGroupIds(next);
    if (next.size > 0) setGroupError(undefined);
  };

  const handleSubmit = () => {
    if (flowId === undefined) return;
    if (editsGroups && groupIds.size === 0) {
      setGroupError(m.location_mfa_override_groups_required());
      return;
    }
    onSubmit(flowId, [...groupIds]);
    onClose();
  };

  return (
    <Modal
      title={title}
      id="mfa-flow-assignment-modal"
      contentClassName="mfa-flow-assignment-modal"
      isOpen={isOpen}
      onClose={onClose}
    >
      {onGroupStep ? (
        <>
          <SelectionSection
            options={groupOptions ?? []}
            selection={groupIds}
            onChange={handleGroupChange}
            searchPlaceholder={m.controls_search()}
            visibleItemsLimit={10}
          />
          <FieldError error={groupError} />
        </>
      ) : (
        <SelectionSection
          options={flowSelectionOptions}
          selection={flowId === undefined ? new Set<number>() : new Set([flowId])}
          onChange={handleFlowChange}
          renderItem={renderMfaFlowSelectionItem}
          searchPlaceholder={m.controls_search()}
          showActions={false}
          enableDividers
          itemGap={12}
          visibleItemsLimit={4}
        />
      )}
      <SizedBox height={ThemeSpacing.Xl} />
      <Controls>
        {onGroupStep && (
          <Button
            variant="outlined"
            text={m.controls_back()}
            onClick={() => setOnGroupStep(false)}
          />
        )}
        <div className="right">
          <Button variant="secondary" text={m.controls_cancel()} onClick={onClose} />
          {editsGroups && !onGroupStep ? (
            <Button
              text={m.controls_continue()}
              disabled={flowId === undefined}
              onClick={() => setOnGroupStep(true)}
            />
          ) : (
            <Button
              text={m.controls_submit()}
              disabled={flowId === undefined}
              onClick={handleSubmit}
            />
          )}
        </div>
      </Controls>
    </Modal>
  );
};
