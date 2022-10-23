import './style.scss';

import { HTMLMotionProps, motion } from 'framer-motion';
import React from 'react';

/**
 * Styled to hold one icon in standardized container.
 *
 * Pass `blank' as className to get plain icon.
 */
const IconButton = React.forwardRef<
  HTMLButtonElement,
  HTMLMotionProps<'button'>
>(({ children, className, ...rest }, ref) => {
  return (
    <motion.button
      className={className ? `icon-button ${className}` : 'icon-button'}
      ref={ref}
      {...rest}
    >
      {children}
    </motion.button>
  );
});

export default IconButton;
