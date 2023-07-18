import './style.scss';

import classNames from 'classnames';
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

const Badge: React.FC<Props & HTMLMotionProps<'span'>> = ({
  text,
  styleVariant = BadgeStyleVariant.STANDARD,
  className,
  ...rest
}) => {
  const cn = useMemo(
    () => classNames('badge', className, styleVariant.valueOf()),
    [className, styleVariant],
  );

  const currentVariant = useMemo(() => styleVariant, [styleVariant]);

  return (
    <motion.span
      className={cn}
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
