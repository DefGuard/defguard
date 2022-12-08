import classNames from 'classnames';
import { motion } from 'framer-motion';
import { ReactNode, useMemo } from 'react';

import SvgIconUserListExpanded from '../../svg/IconUserListExpanded';
import SvgIconUserListHover from '../../svg/IconUserListHover';

interface Props {
  children?: ReactNode;
  expanded?: boolean;
  title: string;
  actions: ReactNode[];
  onChange: () => void;
  disableExpand?: boolean;
}
export const ExpandableCard = ({
  children,
  expanded,
  title,
  actions,
  onChange,
  disableExpand = false,
}: Props) => {
  const cn = useMemo(
    () =>
      classNames('expandable-card', {
        expanded,
      }),
    [expanded]
  );
  return (
    <motion.div className={cn}>
      <div className="top">
        <button
          type="button"
          onClick={() => {
            if (!disableExpand && onChange) {
              onChange();
            }
          }}
          className="expand-button"
        >
          {expanded ? <SvgIconUserListExpanded /> : <SvgIconUserListHover />}
          <span>{title}</span>
        </button>
        <div className="actions">{actions}</div>
      </div>
      {children && expanded ? (
        <div className="expanded-content">{children}</div>
      ) : null}
    </motion.div>
  );
};
