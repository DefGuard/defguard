import './style.scss';

import classNames from 'classnames';
import { motion, Variant, Variants } from 'framer-motion';
import { isUndefined } from 'lodash-es';
import { ReactNode, useMemo, useState } from 'react';

import { cardsShadow, inactiveBoxShadow } from '../../../constants';
import SvgIconUserListExpanded from '../../svg/IconUserListExpanded';
import SvgIconUserListHover from '../../svg/IconUserListHover';

interface Props {
  children?: ReactNode;
  expanded?: boolean;
  title: string;
  actions?: ReactNode[];
  onChange?: () => void;
  disableExpand?: boolean;
  topExtras?: ReactNode;
}

export const ExpandableCard = ({
  children,
  title,
  actions,
  onChange,
  expanded,
  topExtras,
  disableExpand = false,
}: Props) => {
  const cn = useMemo(
    () =>
      classNames('expandable-card', {
        expanded,
      }),
    [expanded]
  );

  const controlledOutside = useMemo(() => !isUndefined(expanded), [expanded]);

  const [localExpanded, setLocalExpanded] = useState(false);
  const [hovered, setHovered] = useState(false);

  return (
    <motion.div
      className={cn}
      variants={containerVariants}
      custom={{ hovered }}
      animate={expanded ? 'expanded' : 'idle'}
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
    >
      <div className="top">
        <button
          type="button"
          onClick={() => {
            if (!disableExpand && controlledOutside && onChange) {
              onChange();
            }
            if (!disableExpand && !controlledOutside) {
              setLocalExpanded((state) => !state);
            }
          }}
          className="expand-button"
        >
          {expanded ? <SvgIconUserListExpanded /> : <SvgIconUserListHover />}
          <span>{title}</span>
        </button>
        {!isUndefined(topExtras) && <div className="extras">{topExtras}</div>}
        {actions && <div className="actions">{actions}</div>}
      </div>
      {children && (controlledOutside ? expanded : localExpanded) ? (
        <div className="expanded-content">{children}</div>
      ) : null}
    </motion.div>
  );
};

type ContainerCustom = {
  hovered?: boolean;
};

const containerVariants: Variants = {
  idle: ({ hovered }: ContainerCustom) => {
    const res: Variant = {
      boxShadow: inactiveBoxShadow,
    };

    if (hovered) {
      res.boxShadow = cardsShadow;
    }

    return res;
  },
  expanded: {
    boxShadow: cardsShadow,
  },
};
