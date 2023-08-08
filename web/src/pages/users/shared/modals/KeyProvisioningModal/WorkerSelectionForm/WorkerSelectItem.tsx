import { AnimatePresence, motion, Variants } from 'framer-motion';
import React, { useMemo } from 'react';

import SvgIconCheckmarkGreen from '../../../../../../shared/components/svg/IconCheckmarkGreen';
import SvgIconConnected from '../../../../../../shared/components/svg/IconConnected';
import SvgIconDisconnected from '../../../../../../shared/components/svg/IconDisconnected';
import { ColorsRGB } from '../../../../../../shared/constants';
import { DeviceAvatar } from '../../../../../../shared/defguard-ui/components/Layout/DeviceAvatar/DeviceAvatar';

interface Props {
  onChange: (v: string) => void;
  value?: string;
  expected: string;
  active?: boolean;
}

const selectVariants: Variants = {
  idle: {
    borderColor: ColorsRGB.GrayBorder,
  },
  selected: {
    borderColor: ColorsRGB.Success,
  },
  disabled: {
    borderColor: ColorsRGB.GrayBorder,
    opacity: 0.5,
  },
};

const avatarIconsVariants: Variants = {
  initial: {
    opacity: 0,
    scale: 0.5,
    x: '-50%',
    y: '-50%',
    top: '50%',
    left: '50%',
  },
  animate: {
    opacity: 1,
    scale: 1,
    x: '-50%',
    y: '-50%',
    top: '50%',
    left: '50%',
  },
  exit: {
    opacity: 0,
    scale: 0.5,
    x: '-50%',
    y: '-50%',
    top: '50%',
    left: '50%',
  },
};

const avatarBoxVariants: Variants = {
  idle: {
    backgroundColor: ColorsRGB.Primary,
  },
  selected: {
    backgroundColor: ColorsRGB.Success,
  },
  disabled: {
    backgroundColor: ColorsRGB.GrayLight,
  },
};

const statusTextVariants: Variants = {
  active: {
    color: ColorsRGB.SuccessDark,
  },
  inactive: {
    color: ColorsRGB.GrayLight,
  },
};

const WorkerSelectItem = React.forwardRef<HTMLInputElement, Props>(
  ({ value, onChange, expected, active = false, ...rest }, ref) => {
    const getVariant: string = useMemo(() => {
      if (!active) {
        return 'disabled';
      }
      if (value === expected) {
        return 'selected';
      }
      return 'idle';
    }, [active, value, expected]);

    return (
      <motion.div
        className="worker-select"
        onClick={() => {
          if (active) {
            onChange(expected);
          }
        }}
        variants={selectVariants}
        animate={getVariant}
        whileHover={{
          borderColor: ColorsRGB.Primary,
        }}
        layout
      >
        <input ref={ref} {...rest} />
        <motion.div
          variants={avatarBoxVariants}
          animate={getVariant}
          className="avatar-box"
        >
          <AnimatePresence>
            {value !== expected ? (
              <motion.span
                variants={avatarIconsVariants}
                initial="initial"
                animate="animate"
                exit="exit"
                key="avatar"
              >
                <DeviceAvatar />
              </motion.span>
            ) : (
              <motion.span
                variants={avatarIconsVariants}
                initial="initial"
                animate="animate"
                exit="exit"
                className="checkmark"
                key="checkmark"
              >
                <SvgIconCheckmarkGreen />
              </motion.span>
            )}
          </AnimatePresence>
        </motion.div>
        <p className="name">{expected}</p>
        <div className="status">
          <motion.span
            variants={statusTextVariants}
            animate={active ? 'active' : 'inactive'}
          >
            {active ? 'Available' : 'Unavailable'}
          </motion.span>
          {active ? <SvgIconConnected /> : <SvgIconDisconnected />}
        </div>
      </motion.div>
    );
  },
);

export default WorkerSelectItem;
