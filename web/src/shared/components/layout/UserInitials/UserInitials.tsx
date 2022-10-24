import './style.scss';

import { HTMLMotionProps, motion, Variants } from 'framer-motion';
import React from 'react';

import { ColorsRGB } from '../../../constants';
import { extractInitials } from '../../../utils/extractInitials';

export enum UserInitialsType {
  BIG = 'BIG',
  SMALL = 'SMALL',
}

interface Props {
  first_name: string;
  last_name: string;
  type?: UserInitialsType;
}

const InitialsBoxVariants: Variants = {
  rest: (custom: UserInitialsType) => {
    switch (custom) {
      case UserInitialsType.BIG:
        return {
          backgroundColor: ColorsRGB.Primary,
        };
      case UserInitialsType.SMALL:
        return {
          backgroundColor: ColorsRGB.BgLight,
        };
    }
  },
  hover: (custom: UserInitialsType) => {
    switch (custom) {
      case UserInitialsType.BIG:
        return {
          backgroundColor: ColorsRGB.BgLight,
        };
      case UserInitialsType.SMALL:
        return {
          backgroundColor: ColorsRGB.Primary,
        };
    }
  },
};

const InitialsVariants: Variants = {
  rest: (custom: UserInitialsType) => {
    switch (custom) {
      case UserInitialsType.BIG:
        return {
          color: ColorsRGB.White,
        };
      case UserInitialsType.SMALL:
        return {
          color: ColorsRGB.GrayDark,
        };
    }
  },
  hover: (custom: UserInitialsType) => {
    switch (custom) {
      case UserInitialsType.BIG:
        return {
          color: ColorsRGB.GrayDark,
        };
      case UserInitialsType.SMALL:
        return {
          color: ColorsRGB.White,
        };
    }
  },
};

/**
 * Displays styled semi avatar box with user initials as a content.
 * @param first_name first name from User type
 * @param last_name last name from User type
 * @param type Style variant.
 */
const UserInitials: React.FC<Props & HTMLMotionProps<'div'>> = ({
  first_name,
  last_name,
  type = UserInitialsType.BIG,
  ...rest
}) => {
  return (
    <motion.span
      className={
        type === UserInitialsType.BIG
          ? 'user-initials-box big'
          : 'user-initials-box small'
      }
      custom={type}
      variants={InitialsBoxVariants}
      initial="rest"
      animate="rest"
      whileHover="hover"
      {...rest}
    >
      <motion.span variants={InitialsVariants} custom={type}>
        {extractInitials(`${first_name} ${last_name}`)}
      </motion.span>
    </motion.span>
  );
};

export default UserInitials;
