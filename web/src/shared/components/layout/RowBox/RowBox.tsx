import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps, motion, Variants } from 'framer-motion';
import { ReactNode, useMemo } from 'react';

import { buttonsBoxShadow, ColorsRGB, inactiveBoxShadow } from '../../../constants';

interface Props extends HTMLMotionProps<'div'> {
  children: ReactNode;
  className?: string;
  disabled?: boolean;
}

export const RowBox = ({ children, className, disabled = false, ...rest }: Props) => {
  const cn = useMemo(
    () =>
      classNames('row-box', className, {
        disabled: disabled,
      }),
    [className, disabled]
  );
  return (
    <motion.div
      className={cn}
      initial="rowBoxIdle"
      whileHover="rowBoxActive"
      variants={defaultVariants}
      custom={disabled}
      {...rest}
    >
      {children}
    </motion.div>
  );
};

const defaultVariants: Variants = {
  rowBoxIdle: ({ disabled }) => ({
    borderColor: ColorsRGB.GrayBorder,
    boxShadow: inactiveBoxShadow,
    opacity: disabled ? 0.8 : 1,
  }),
  rowBoxActive: ({ disabled }) => ({
    borderColor: ColorsRGB.GrayLighter,
    boxShadow: disabled ? inactiveBoxShadow : buttonsBoxShadow,
    opacity: disabled ? 0.8 : 1,
  }),
};
