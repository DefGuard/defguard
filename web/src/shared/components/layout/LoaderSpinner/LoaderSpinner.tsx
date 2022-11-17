import './style.scss';

import { HTMLMotionProps, motion, useAnimation, Variants } from 'framer-motion';
import { useEffect } from 'react';

import { ColorsRGB } from '../../../constants';

interface Props extends HTMLMotionProps<'span'> {
  size?: number;
  backColor?: ColorsRGB;
  frontColor?: ColorsRGB;
}

/**
 * Creates infinitely spinning circle.
 */
const LoaderSpinner = ({
  size = 20,
  backColor = ColorsRGB.Transparent,
  frontColor = ColorsRGB.Primary,
  ...rest
}: Props) => {
  const controls = useAnimation();

  useEffect(() => {
    controls.start('spin');
  }, [controls]);

  return (
    <motion.span
      className="loader-spinner"
      initial="initial"
      variants={variants}
      animate={controls}
      style={{
        height: size,
        width: size,
        borderTop: `3px solid ${frontColor}`,
        borderRight: `3px solid ${backColor}`,
      }}
      {...rest}
    ></motion.span>
  );
};

const variants: Variants = {
  initial: {
    rotate: 0,
  },
  spin: {
    rotate: 360,
    transition: {
      repeat: Infinity,
      type: 'tween',
      repeatDelay: 0,
      ease: 'linear',
      duration: 1,
    },
  },
};

export default LoaderSpinner;
