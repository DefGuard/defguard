import './style.scss';

import { motion, Variants } from 'framer-motion';
import React, { useState } from 'react';

import IconButton from '../../../../../shared/components/layout/IconButton/IconButton';
import { ColorsRGB } from '../../../../../shared/constants';
import AddButtonIcon from './AddButtonIcon';

interface Props {
  callback: () => void;
  text: string;
}

const boxVariants: Variants = {
  idle: {
    borderColor: ColorsRGB.GrayBorder,
  },
  hover: {
    borderColor: ColorsRGB.GrayLighter,
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

const AddComponentBox: React.FC<Props> = ({ callback, text }) => {
  const [hovered, setHovered] = useState(false);

  return (
    <motion.div
      className="add-component"
      initial="idle"
      animate="idle"
      whileHover="hover"
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
      whileTap={{
        scale: 0.9,
      }}
      variants={boxVariants}
      onClick={() => callback()}
    >
      <IconButton
        className="blank"
        variants={buttonVariants}
        initial="idle"
        animate={hovered ? 'hover' : 'idle'}
      >
        <AddButtonIcon rectVariants={iconVariants} hover={hovered} />
      </IconButton>
      <motion.span variants={textVariants}>{text}</motion.span>
    </motion.div>
  );
};

export default AddComponentBox;
