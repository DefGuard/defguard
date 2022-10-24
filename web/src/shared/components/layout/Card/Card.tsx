import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps, motion } from 'framer-motion';
import { ReactNode, useMemo } from 'react';

interface Props extends HTMLMotionProps<'div'> {
  children?: ReactNode;
}

export const Card = ({ children, className, ...rest }: Props) => {
  const cn = useMemo(() => classNames('card', className), [className]);
  return (
    <motion.div className={cn} {...rest}>
      {children}
    </motion.div>
  );
};
