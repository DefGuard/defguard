import './style.scss';

import classNames from 'classnames';
import { motion, TargetAndTransition } from 'framer-motion';
import { useMemo, useState } from 'react';

import { ColorsRGB } from '../../../../../shared/constants';
import { useNavigationStore } from '../../../hooks/useNavigationStore';

export const NavigationCollapse = () => {
  const [hovered, setHovered] = useState(false);
  const isOpen = useNavigationStore((state) => state.isOpen);
  const setState = useNavigationStore((state) => state.setState);
  const getAnimate = useMemo((): TargetAndTransition => {
    const res: TargetAndTransition = {
      borderColor: ColorsRGB.GrayBorder,
      backgroundColor: ColorsRGB.White,
    };
    if (hovered) {
      res.borderColor = ColorsRGB.Primary;
      res.backgroundColor = ColorsRGB.Primary;
    }
    return res;
  }, [hovered]);

  const cn = classNames('navigation-collapse', {
    open: isOpen,
  });

  return (
    <motion.button
      className={cn}
      onClick={() => setState({ isOpen: !isOpen })}
      animate={getAnimate}
      initial={false}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <motion.svg
        xmlns="http://www.w3.org/2000/svg"
        width={22}
        height={22}
        viewBox="0 0 22 22"
        fill="none"
        initial={false}
        animate={{
          rotate: isOpen ? 180 : 0,
        }}
      >
        <motion.g
          animate={{
            fill: hovered ? ColorsRGB.White : ColorsRGB.GrayLight,
          }}
          initial={false}
        >
          <path d="m11.708 7.465 4.243 4.243a1 1 0 0 0 1.414-1.414L13.122 6.05a1 1 0 1 0-1.414 1.414Zm-6 0 4.243 4.243a1 1 0 0 0 1.414-1.414L7.122 6.05a1 1 0 1 0-1.414 1.414Z" />
          <path d="m15.95 10.636-4.243 4.243a1 1 0 0 0 1.414 1.414l4.243-4.243a1 1 0 0 0-1.414-1.414Zm-6 0L5.707 14.88a1 1 0 1 0 1.414 1.414l4.243-4.243a1 1 0 0 0-1.414-1.414Z" />
        </motion.g>
      </motion.svg>
    </motion.button>
  );
};
