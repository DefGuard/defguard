import { type ReactNode, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import type { MfaFlowStepMethods } from '../../../../shared/api/types';
import { MfaFlowStepsTooltip } from '../../../../shared/components/MfaFlowStepsTooltip/MfaFlowStepsTooltip';
import { Chip } from '../../../../shared/defguard-ui/components/Chip/Chip';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { IconKind } from '../../../../shared/defguard-ui/components/Icon';
import { Icon } from '../../../../shared/defguard-ui/components/Icon/Icon';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';

const collapsedChipLimit = 5;

type Props = {
  title: string;
  steps: MfaFlowStepMethods[];
  chips: string[];
  leading: ReactNode;
  editLabel: string;
  onEdit: () => void;
  removeLabel?: string;
  onRemove?: () => void;
};

export const MfaFlowAssignmentCard = ({
  title,
  steps,
  chips,
  leading,
  editLabel,
  onEdit,
  removeLabel,
  onRemove,
}: Props) => {
  const [expanded, setExpanded] = useState(false);
  const foldable = chips.length > collapsedChipLimit;
  const visibleChips = expanded ? chips : chips.slice(0, collapsedChipLimit);

  return (
    <div className="mfa-flow-assignment-card">
      <div className="header">
        {leading}
        <MfaFlowStepsTooltip steps={steps}>
          <span className="flow-title">{title}</span>
        </MfaFlowStepsTooltip>
        <div className="actions">
          <button
            type="button"
            className="card-action"
            aria-label={editLabel}
            onClick={onEdit}
          >
            <Icon icon={IconKind.Edit} size={20} />
          </button>
          {isPresent(onRemove) && (
            <button
              type="button"
              className="card-action"
              aria-label={removeLabel}
              onClick={onRemove}
            >
              <Icon icon={IconKind.Delete} size={20} />
            </button>
          )}
        </div>
      </div>
      <Divider spacing={ThemeSpacing.Md} />
      <div className="chips">
        {visibleChips.map((chip) => (
          <Chip text={chip} key={chip} />
        ))}
      </div>
      {foldable && (
        <button
          type="button"
          className="fold-chips"
          onClick={() => setExpanded((value) => !value)}
        >
          {expanded ? m.controls_show_less() : m.controls_show_more()}
        </button>
      )}
    </div>
  );
};
