import './style.scss';

import classNames from 'classnames';
import { motion, Variant, Variants } from 'framer-motion';
import { useMemo, useState } from 'react';

import { ColorsRGB } from '../../../../../shared/constants';
import AddButtonIcon from './AddButtonIcon';

interface Props {
  callback: () => void;
  text: string;
  disabled?: boolean;
  'data-testid'?: string;
}

export const AddComponentBox = ({
  callback,
  text,
  disabled = false,
  'data-testid': testId,
}: Props) => {
  const [hovered, setHovered] = useState(false);

  const cn = useMemo(
    () => classNames('add-component', { disabled: disabled }),
    [disabled]
  );

  const getAnimate = useMemo(() => {
    if (disabled) {
      return 'idle';
    }
    if (hovered) {
      return 'hover';
    }
    return 'idle';
  }, [disabled, hovered]);

  return (
    <motion.div
      data-testid={testId}
      className={cn}
      initial="idle"
      animate={getAnimate}
      onHoverStart={() => {
        if (!disabled) {
          setHovered(true);
        }
      }}
      onHoverEnd={() => setHovered(false)}
      whileTap={{
        scale: 0.9,
      }}
      variants={boxVariants}
      onClick={() => {
        if (!disabled) {
          callback();
        }
      }}
      custom={{ disabled }}
    >
      <motion.button
        variants={buttonVariants}
        initial="idle"
        animate={hovered ? 'hover' : 'idle'}
      >
        <AddButtonIcon rectVariants={iconVariants} hover={hovered} />
      </motion.button>
      <motion.span variants={textVariants}>{text}</motion.span>
    </motion.div>
  );
};

interface BoxCustom {
  disabled?: boolean;
}

const boxVariants: Variants = {
  idle: ({ disabled }: BoxCustom) => {
    const res: Variant = {
      borderColor: ColorsRGB.GrayBorder,
      opacity: 1,
    };
    if (disabled) {
      res.opacity = 0.4;
    }
    return res;
  },
  hover: ({ disabled }: BoxCustom) => {
    const res: Variant = {
      borderColor: ColorsRGB.GrayLighter,
      opacity: 1,
    };
    if (disabled) {
      res.opacity = 0.4;
    }
    return res;
  },
};

const buttonVariants: Variants = {
  idle: {
    backgroundColor: ColorsRGB.White,
  },
  hover: {
    backgroundColor: ColorsRGB.Primary,
  },
};

const iconVariants: Variants = {
  idle: {
    fill: ColorsRGB.GrayLight,
  },
  hover: {
    fill: ColorsRGB.White,
  },
};

const textVariants: Variants = {
  idle: {
    color: ColorsRGB.GrayDarker,
  },
  hover: {
    color: ColorsRGB.Primary,
  },
};
