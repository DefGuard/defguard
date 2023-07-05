import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps, motion } from 'framer-motion';
import { ReactNode, useMemo } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { deviceBreakpoints } from '../../../constants';

interface Props extends HTMLMotionProps<'div'> {
  children?: ReactNode;
  shaded?: boolean;
  hideMobile?: boolean;
}

export const Card = ({ children, className, shaded, hideMobile, ...rest }: Props) => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const cn = useMemo(
    () =>
      classNames('card', className, {
        shaded,
        'hide-appearance': breakpoint !== 'desktop' && hideMobile,
      }),
    [breakpoint, className, hideMobile, shaded]
  );

  return (
    <motion.div className={cn} {...rest}>
      {children}
    </motion.div>
  );
};
