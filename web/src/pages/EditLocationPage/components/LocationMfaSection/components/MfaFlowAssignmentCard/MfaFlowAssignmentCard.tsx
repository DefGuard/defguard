import './style.scss';

import useResizeObserver from '@react-hook/resize-observer';
import {
  type ReactNode,
  type RefObject,
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
} from 'react';
import { m } from '../../../../../../paraglide/messages';
import type { MfaFlowStepMethods } from '../../../../../../shared/api/types';
import { MfaFlowStepsTooltip } from '../../../../../../shared/components/MfaFlowStepsTooltip/MfaFlowStepsTooltip';
import { Chip } from '../../../../../../shared/defguard-ui/components/Chip/Chip';
import { Divider } from '../../../../../../shared/defguard-ui/components/Divider/Divider';
import { IconKind } from '../../../../../../shared/defguard-ui/components/Icon';
import { Icon } from '../../../../../../shared/defguard-ui/components/Icon/Icon';
import { InfoBanner } from '../../../../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { ThemeSpacing } from '../../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../../shared/defguard-ui/utils/isPresent';

const collapsedRowCount = 2;

const useCollapsedHeight = (target: RefObject<HTMLElement | null>) => {
  const [collapsedHeight, setCollapsedHeight] = useState<number | null>(null);

  const measure = useCallback(() => {
    const container = target.current;
    if (!container) return;
    const containerTop = container.getBoundingClientRect().top;
    let rows = 0;
    let rowTop = Number.NEGATIVE_INFINITY;
    let rowsBottom = 0;
    for (const child of container.children) {
      const rect = child.getBoundingClientRect();
      if (rect.top > rowTop + 1) {
        rows += 1;
        rowTop = rect.top;
      }
      if (rows > collapsedRowCount) {
        setCollapsedHeight(rowsBottom);
        return;
      }
      rowsBottom = Math.max(rowsBottom, rect.bottom - containerTop);
    }
    setCollapsedHeight(null);
  }, [target]);

  useLayoutEffect(() => {
    measure();
  }, [measure]);

  useResizeObserver(target, measure);

  return collapsedHeight;
};

type Props = {
  title: string;
  steps: MfaFlowStepMethods[];
  chips: string[];
  leading: ReactNode;
  editLabel: string;
  onEdit: () => void;
  removeLabel?: string;
  onRemove?: () => void;
  unavailableText?: string;
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
  unavailableText,
}: Props) => {
  const [expanded, setExpanded] = useState(false);
  const chipsRef = useRef<HTMLDivElement>(null);
  const collapsedHeight = useCollapsedHeight(chipsRef);
  const foldable = isPresent(collapsedHeight);

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
      <div
        className="chips"
        ref={chipsRef}
        style={foldable && !expanded ? { maxHeight: collapsedHeight } : undefined}
      >
        {chips.map((chip) => (
          <Chip text={chip} key={chip} />
        ))}
      </div>
      {isPresent(unavailableText) && (
        <InfoBanner icon="lock-closed" variant="warning" text={unavailableText} />
      )}
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
