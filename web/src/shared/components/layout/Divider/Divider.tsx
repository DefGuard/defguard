import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps, motion } from 'framer-motion';
import { useMemo } from 'react';

interface Props extends HTMLMotionProps<'div'> {
  direction?: DividerDirection;
}

/**
 * Styled empty div, serves as visual divider of content.
 */
export const Divider = ({
  className,
  direction = DividerDirection.HORIZONTAL,
  ...rest
}: Props) => {
  const cn = useMemo(
    () => classNames('divider', className, direction?.valueOf()),
    [className, direction],
  );

  return <motion.div {...rest} className={cn} />;
};

export enum DividerDirection {
  HORIZONTAL = 'horizontal',
  VERTICAL = 'vertical',
}
