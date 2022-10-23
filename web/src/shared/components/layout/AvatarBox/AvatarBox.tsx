import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps, motion } from 'framer-motion';
import { ReactNode, useMemo } from 'react';

export interface AvatarBoxProps extends HTMLMotionProps<'div'> {
  children?: ReactNode;
}
export const AvatarBox = ({ children, className, ...rest }: AvatarBoxProps) => {
  const cn = useMemo(() => classNames('avatar-box', className), [className]);
  return (
    <motion.div className={cn} {...rest}>
      {children}
    </motion.div>
  );
};
