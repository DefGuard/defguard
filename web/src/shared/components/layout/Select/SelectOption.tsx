import classNames from 'classnames';
import { HTMLMotionProps, motion } from 'framer-motion';
import { useMemo, useState } from 'react';

import { ColorsRGB } from '../../../constants';

export interface SelectOptionProps extends HTMLMotionProps<'button'> {
  label?: string;
  selected?: boolean;
  // Marks said option as the one that make new options
  createOption?: boolean;
}

export const SelectOption = ({
  label,
  selected,
  className,
  createOption,
  ...rest
}: SelectOptionProps) => {
  const [hovered, setHovered] = useState(false);
  const cn = useMemo(
    () =>
      classNames('select-option', className, {
        hovered,
        selected,
        'create-option': createOption,
      }),
    [className, hovered, selected, createOption]
  );

  const getAnimate = useMemo(() => {
    const res = {
      backgroundColor: ColorsRGB.White,
      color: ColorsRGB.GrayDark,
    };
    if (hovered) {
      res.backgroundColor = ColorsRGB.BgLight;
    }
    if (createOption) {
      res.color = ColorsRGB.Primary;
      return res;
    }
    if (selected) {
      res.backgroundColor = ColorsRGB.BgLight;
      res.color = ColorsRGB.TextMain;
    }
    return res;
  }, [createOption, hovered, selected]);

  return (
    <motion.button
      {...rest}
      type="button"
      className={cn}
      initial={false}
      animate={getAnimate}
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
    >
      <span>{label}</span>
    </motion.button>
  );
};
