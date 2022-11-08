import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps, motion } from 'framer-motion';
import { ReactNode, useMemo } from 'react';

interface Props extends HTMLMotionProps<'div'> {
  children?: ReactNode;
  header: string;
}

export const CardWithHeader = ({
  children,
  header,
  className,
  ...rest
}: Props) => {
  const cn = useMemo(() => classNames('card', className), [className]);
  const headerCn = useMemo(() => classNames('header', className), [className]);
  return (
    <>
      <motion.div className={cn} {...rest}>
        <motion.div className={headerCn} {...rest}>
          {header}
        </motion.div>
        {children}
      </motion.div>
    </>
  );
};
