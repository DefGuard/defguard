import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps, motion, Variants } from 'framer-motion';
import { ReactNode, useMemo } from 'react';

import {
  buttonsBoxShadow,
  ColorsRGB,
  inactiveBoxShadow,
} from '../../../constants';

interface Props extends HTMLMotionProps<'div'> {
  children: ReactNode;
  className?: string;
}

export const RowBox = ({ children, className, ...rest }: Props) => {
  const cn = useMemo(() => classNames('row-box', className), [className]);
  return (
    <motion.div
      className={cn}
      initial="rowBoxIdle"
      whileHover="rowBoxActive"
      variants={defaultVariants}
      {...rest}
    >
      {children}
    </motion.div>
  );
};

const defaultVariants: Variants = {
  rowBoxIdle: {
    borderColor: ColorsRGB.GrayBorder,
    boxShadow: inactiveBoxShadow,
  },
  rowBoxActive: {
    borderColor: ColorsRGB.GrayLighter,
    boxShadow: buttonsBoxShadow,
  },
};
