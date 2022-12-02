import './style.scss';

import { HTMLMotionProps, motion, Variants } from 'framer-motion';
import React, { useMemo } from 'react';

import { ColorsRGB } from '../../../constants';

export enum BadgeStyleVariant {
  STANDARD = 'standard',
  WARNING = 'warning',
  INACTIVE = 'inactive',
  SUCCESS = 'success',
  PRIMARY = 'primary',
}

interface Props {
  text: string;
  styleVariant?: BadgeStyleVariant;
}

/**
 * Styled badge element.
 *
 * Displays content inside span.
 * @param text String to display as a badge.
 */
const Badge: React.FC<Props & HTMLMotionProps<'span'>> = ({
  text,
  styleVariant = BadgeStyleVariant.STANDARD,
  className,
  ...rest
}) => {
  const getClassName = useMemo(() => {
    let res: string | string[] = ['badge'];
    res.push(styleVariant.valueOf());
    res = res.join(' ');
    if (className && className.length) {
      return `${res} ${className}`;
    }
    return res;
  }, [styleVariant, className]);

  const currentVariant = useMemo(() => styleVariant, [styleVariant]);

  return (
    <motion.span
      className={getClassName}
      {...rest}
      variants={badgeVariants}
      animate={currentVariant}
      initial={false}
    >
      {text}
    </motion.span>
  );
};
export default Badge;

const badgeVariants: Variants = {
  primary: {
    backgroundColor: ColorsRGB.Primary,
    color: ColorsRGB.White,
  },
  standard: {
    backgroundColor: ColorsRGB.BgLight,
    color: ColorsRGB.GrayLight,
  },
  warning: {
    backgroundColor: ColorsRGB.LightRedBg,
    color: ColorsRGB.Error,
  },
  inactive: {
    backgroundColor: ColorsRGB.GrayLight,
    color: ColorsRGB.White,
  },
  success: {
    backgroundColor: ColorsRGB.SuccessDark,
    color: ColorsRGB.White,
  },
};
