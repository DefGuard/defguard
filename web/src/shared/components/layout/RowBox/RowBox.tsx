import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps, motion, TargetAndTransition } from 'framer-motion';
import { ReactNode, useMemo, useState } from 'react';

import { buttonsBoxShadow, ColorsRGB, inactiveBoxShadow } from '../../../constants';

interface Props extends HTMLMotionProps<'div'> {
  children: ReactNode;
  className?: string;
  disabled?: boolean;
  customAnimate?: TargetAndTransition;
}

export const RowBox = ({
  children,
  className,
  customAnimate,
  disabled = false,
  ...rest
}: Props) => {
  const [hovered, setHovered] = useState(false);
  const cn = useMemo(
    () =>
      classNames('row-box', className, {
        disabled: disabled,
      }),
    [className, disabled],
  );

  const getAnimate = useMemo((): TargetAndTransition => {
    let res: TargetAndTransition = {
      borderColor: ColorsRGB.GrayBorder,
      boxShadow: inactiveBoxShadow,
      opacity: 1,
    };
    if (disabled) {
      res.opacity = 0.8;
    }
    if (hovered) {
      res.boxShadow = buttonsBoxShadow;
    }
    if (customAnimate) {
      res = { ...res, ...customAnimate };
    }
    return res;
  }, [disabled, hovered, customAnimate]);

  return (
    <motion.div
      className={cn}
      initial={false}
      animate={getAnimate}
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
      {...rest}
    >
      {children}
    </motion.div>
  );
};
