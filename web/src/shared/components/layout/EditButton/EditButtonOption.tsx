import { HTMLMotionProps, motion, Variant, Variants } from 'framer-motion';
import { useState } from 'react';

import { ColorsRGB } from '../../../constants';

export enum EditButtonOptionStyleVariant {
  STANDARD = 'STANDARD',
  WARNING = 'WARNING',
}

interface EditButtonOptionProps extends HTMLMotionProps<'button'> {
  text: string;
  styleVariant?: EditButtonOptionStyleVariant;
}

export const EditButtonOption = ({
  text,
  disabled,
  styleVariant = EditButtonOptionStyleVariant.STANDARD,
  ...rest
}: EditButtonOptionProps) => {
  const [hovered, setHovered] = useState(false);
  return (
    <motion.button
      disabled={disabled}
      variants={editButtonOptionVariants}
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
      animate={hovered ? 'optionHover' : 'optionIdle'}
      initial="optionIdle"
      custom={{ styleVariant, disabled }}
      {...rest}
    >
      {text}
    </motion.button>
  );
};

interface EditButtonOptionVariantCustom {
  styleVariant: EditButtonOptionStyleVariant;
  disabled?: boolean;
}

const disabledVariant: Variant = {
  color: ColorsRGB.GrayDark,
  backgroundColor: ColorsRGB.White,
  opacity: 0.8,
};

const editButtonOptionVariants: Variants = {
  optionIdle: ({ styleVariant, disabled }: EditButtonOptionVariantCustom) => {
    if (disabled) {
      return disabledVariant;
    }
    const res = {
      color: ColorsRGB.GrayDark,
      backgroundColor: ColorsRGB.White,
      opacity: 1,
    };
    if (styleVariant === EditButtonOptionStyleVariant.WARNING) {
      res.color = ColorsRGB.Error;
    }
    return res;
  },
  optionHover: ({ styleVariant, disabled }: EditButtonOptionVariantCustom) => {
    if (disabled) {
      return disabledVariant;
    }
    const res = {
      color: ColorsRGB.TextMain,
      backgroundColor: ColorsRGB.BgLight,
      opacity: 1,
    };
    if (styleVariant === EditButtonOptionStyleVariant.WARNING) {
      res.color = ColorsRGB.ErrorDark;
    }
    return res;
  },
};
