import { motion, Variants } from 'framer-motion';
import React from 'react';

interface Props {
  rectVariants: Variants;
  hover: boolean;
}

const AddButtonIcon: React.FC<Props> = ({ rectVariants, hover }) => {
  return (
    <motion.svg xmlns="http://www.w3.org/2000/svg" width={22} height={22}>
      <defs>
        <style>{'\n.icon-plus-gray_svg__a{opacity:0}\n'}</style>
      </defs>
      <g className="icon-plus-gray_svg__b">
        <motion.rect
          className="icon-plus-gray_svg__c"
          width={10}
          height={2}
          rx={1}
          transform="rotate(-90 13 3)"
          variants={rectVariants}
          animate={hover ? 'hover' : 'idle'}
        />
        <motion.rect
          className="icon-plus-gray_svg__c"
          width={10}
          height={2}
          rx={1}
          transform="rotate(-180 8 6)"
          variants={rectVariants}
        />
      </g>
    </motion.svg>
  );
};

export default AddButtonIcon;
