import classNames from 'classnames';
import { HTMLMotionProps, motion, Variants } from 'framer-motion';
import { useMemo, useState } from 'react';

import { ColorsRGB } from '../../../constants';

export interface SelectOptionProps extends HTMLMotionProps<'button'> {
  label: string;
  selected?: boolean;
}

export const SelectOption = ({
  label,
  selected,
  className,
  ...rest
}: SelectOptionProps) => {
  const [hovered, setHovered] = useState(false);
  const cn = useMemo(() => classNames('select-option', className), [className]);

  const getAnimate = useMemo(() => {
    if (selected || hovered) {
      return 'active';
    }
    return 'idle';
  }, [hovered, selected]);

  return (
    <motion.button
      {...rest}
      type="button"
      className={cn}
      animate={getAnimate}
      variants={selectOptionVariants}
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
    >
      <span>{label}</span>
    </motion.button>
  );
};

const selectOptionVariants: Variants = {
  idle: {
    backgroundColor: ColorsRGB.White,
    color: ColorsRGB.GrayDark,
  },
  active: {
    backgroundColor: ColorsRGB.BgLight,
    color: ColorsRGB.TextMain,
  },
};
