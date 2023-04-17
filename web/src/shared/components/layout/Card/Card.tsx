import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps, motion } from 'framer-motion';
import { ReactNode, useMemo } from 'react';

interface Props extends HTMLMotionProps<'div'> {
  children?: ReactNode;
  shaded?: boolean;
}

export const Card = ({ children, className, shaded, ...rest }: Props) => {
  const cn = useMemo(
    () =>
      classNames('card', className, {
        shaded,
      }),
    [className, shaded]
  );

  return (
    <motion.div className={cn} {...rest}>
      {children}
    </motion.div>
  );
};
