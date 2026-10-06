import './style.scss';

import { Reorder, useDragControls } from 'motion/react';
import { type PointerEvent, type RefObject, useState } from 'react';
import { m } from '../../../../../../paraglide/messages';
import type {
  MfaFlowAssignment,
  MfaFlowStepMethods,
} from '../../../../../../shared/api/types';
import { Icon } from '../../../../../../shared/defguard-ui/components/Icon/Icon';
import { isPresent } from '../../../../../../shared/defguard-ui/utils/isPresent';
import { MfaFlowAssignmentCard } from '../MfaFlowAssignmentCard/MfaFlowAssignmentCard';

type Props = {
  override: MfaFlowAssignment;
  position: number;
  title: string;
  steps: MfaFlowStepMethods[];
  chips: string[];
  dragConstraints: RefObject<HTMLUListElement | null>;
  onEdit: () => void;
  onRemove: () => void;
  unavailableText?: string;
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
  unavailableText,
}: Props) => {
  const dragControls = useDragControls();
  const [isDragging, setDragging] = useState(false);

  const startDrag = (event: PointerEvent<HTMLButtonElement>) => {
    setDragging(true);
    const listeners = new AbortController();
    const stopDrag = () => {
      listeners.abort();
      setDragging(false);
    };
    window.addEventListener('pointerup', stopDrag, { signal: listeners.signal });
    window.addEventListener('pointercancel', stopDrag, { signal: listeners.signal });
    dragControls.start(event);
  };

  return (
    <Reorder.Item
      value={override}
      dragListener={false}
      dragControls={dragControls}
      // Motion offsets resting items when the constraints element resizes, so constrain only while dragging.
      dragConstraints={isDragging ? dragConstraints : undefined}
      dragElastic={false}
      layout="position"
      className="assignment-row mfa-flow-override-row"
    >
      <span className="marker">
        {isPresent(unavailableText) ? <Icon icon="disabled" size={16} /> : position}
      </span>
      <MfaFlowAssignmentCard
        title={title}
        steps={steps}
        chips={chips}
        unavailableText={unavailableText}
        leading={
          <button
            type="button"
            className="drag-button"
            aria-label={m.location_mfa_override_reorder({ number: position })}
            onPointerDown={startDrag}
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
