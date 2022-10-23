import { HTMLMotionProps, motion, Variants } from 'framer-motion';
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
      custom={{ styleVariant }}
      {...rest}
    >
      {text}
    </motion.button>
  );
};

interface EditButtonOptionVariantCustom {
  styleVariant: EditButtonOptionStyleVariant;
}

const editButtonOptionVariants: Variants = {
  optionIdle: ({ styleVariant }: EditButtonOptionVariantCustom) => ({
    color:
      styleVariant === EditButtonOptionStyleVariant.WARNING
        ? ColorsRGB.Error
        : ColorsRGB.GrayDark,
    backgroundColor: ColorsRGB.White,
  }),
  optionHover: ({ styleVariant }: EditButtonOptionVariantCustom) => ({
    color:
      styleVariant === EditButtonOptionStyleVariant.WARNING
        ? ColorsRGB.ErrorDark
        : ColorsRGB.TextMain,
    backgroundColor: ColorsRGB.BgLight,
  }),
};
