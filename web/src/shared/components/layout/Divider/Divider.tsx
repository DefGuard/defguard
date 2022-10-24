import './style.scss';

import { HTMLMotionProps, motion } from 'framer-motion';
import { useMemo } from 'react';

interface Props extends HTMLMotionProps<'div'> {
  direction?: DividerDirection;
}

/**
 * Styled empty div, serves as visual divider of content.
 *
 * If parent has margins or paddings that cause it to be smaller then intent it should be given negative margin or adjusted width to match desired width.
 */
const Divider = ({
  className,
  direction = DividerDirection.HORIZONTAL,
  ...rest
}: Props) => {
  const getClassName = useMemo(() => {
    const res = ['divider'];
    if (className) {
      res.push(className);
    }
    res.push(direction?.valueOf());
    return res.join(' ');
  }, [className, direction]);

  return <motion.div {...rest} className={getClassName} />;
};

export default Divider;

export enum DividerDirection {
  HORIZONTAL = 'horizontal',
  VERTICAL = 'vertical',
}
