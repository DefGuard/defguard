import { Reorder, useDragControls } from 'motion/react';
import type { RefObject } from 'react';
import { m } from '../../../../paraglide/messages';
import type { MfaFlowAssignment, MfaFlowStepMethods } from '../../../../shared/api/types';
import { Icon } from '../../../../shared/defguard-ui/components/Icon/Icon';
import { MfaFlowAssignmentCard } from './MfaFlowAssignmentCard';

type Props = {
  override: MfaFlowAssignment;
  position: number;
  title: string;
  steps: MfaFlowStepMethods[];
  chips: string[];
  dragConstraints: RefObject<HTMLUListElement | null>;
  onEdit: () => void;
  onRemove: () => void;
};

export const MfaFlowOverrideRow = ({
  override,
  position,
  title,
  steps,
  chips,
  dragConstraints,
  onEdit,
  onRemove,
}: Props) => {
  const dragControls = useDragControls();

  return (
    <Reorder.Item
      value={override}
      dragListener={false}
      dragControls={dragControls}
      dragConstraints={dragConstraints}
      dragElastic={false}
      layout="position"
      className="assignment-row"
    >
      <span className="marker">{position}</span>
      <MfaFlowAssignmentCard
        title={title}
        steps={steps}
        chips={chips}
        leading={
          <button
            type="button"
            className="drag-button"
            aria-label={m.location_mfa_override_reorder({ number: position })}
            onPointerDown={(event) => dragControls.start(event)}
          >
            <Icon icon="dnd" size={20} />
          </button>
        }
        editLabel={m.location_mfa_override_edit()}
        onEdit={onEdit}
        removeLabel={m.location_mfa_override_remove()}
        onRemove={onRemove}
      />
    </Reorder.Item>
  );
};
