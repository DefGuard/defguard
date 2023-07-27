import classNames from 'classnames';
import { motion, TargetAndTransition } from 'framer-motion';
import { useMemo, useState } from 'react';

import { ColorsRGB } from '../../../../constants';
import { CardTabProps } from '../types';

export const CardTab = ({ onClick, content, active = false }: CardTabProps) => {
  const [hovered, setHovered] = useState(false);
  const cn = useMemo(
    () => classNames('card-tab', { active, hovered }),
    [active, hovered]
  );

  const renderContent = useMemo(() => {
    if (typeof content === 'string') {
      return <span>{content}</span>;
    }
    return content;
  }, [content]);

  const getAnimate = useMemo((): TargetAndTransition => {
    const res: TargetAndTransition = {
      height: 32,
      backgroundColor: ColorsRGB.GrayLighter,
      color: ColorsRGB.GrayLight,
    };

    if (active || hovered) {
      res.height = 42;
      res.color = ColorsRGB.TextMain;
      res.backgroundColor = ColorsRGB.White;
    }

    return res;
  }, [active, hovered]);

  return (
    <motion.button
      initial={false}
      animate={getAnimate}
      className={cn}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={onClick}
    >
      {renderContent}
    </motion.button>
  );
};
