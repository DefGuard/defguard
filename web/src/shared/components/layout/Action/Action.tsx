import './style.scss';

import { HTMLMotionProps, motion, Variants } from 'framer-motion';
import { ReactNode, useMemo } from 'react';

import { ColorsRGB } from '../../../constants';

interface Props extends HTMLMotionProps<'span'> {
  children: ReactNode;
}
export const Action = ({ children, className, ...rest }: Props) => {
  const getClassName = useMemo(() => {
    const res = ['action'];
    if (className) {
      res.push(className);
    }
    return res.join(' ');
  }, [className]);

  return (
    <motion.span
      className={getClassName}
      variants={variants}
      whileHover="active"
      initial="idle"
      {...rest}
    >
      {children}
    </motion.span>
  );
};

const variants: Variants = {
  idle: {
    color: ColorsRGB.GrayLight,
  },
  active: {
    color: ColorsRGB.Primary,
  },
};
