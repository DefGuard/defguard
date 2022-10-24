import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps, motion } from 'framer-motion';
import { useMemo } from 'react';

interface Props extends HTMLMotionProps<'label'> {
  colon?: boolean;
}

export const Label = ({
  children,
  className,
  colon = true,
  ...rest
}: Props) => {
  const cn = useMemo(() => classNames(className), [className]);

  return (
    <motion.label className={cn} {...rest}>
      {children}
      {colon ? ':' : null}
    </motion.label>
  );
};
