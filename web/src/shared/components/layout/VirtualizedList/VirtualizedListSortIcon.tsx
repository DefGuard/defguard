import { motion, SVGMotionProps } from 'framer-motion';

import { ColorsRGB } from '../../../constants';

type Props = SVGMotionProps<SVGSVGElement>;

export const VirtualizedListSortIcon = (props: Props) => {
  return (
    <motion.svg
      xmlns="http://www.w3.org/2000/svg"
      width={22}
      height={22}
      {...props}
    >
      <motion.g transform="rotate(-90 11 11)">
        <rect
          width={8}
          height={2}
          rx={1}
          transform="rotate(45 -7.4 14.862)"
          fill={ColorsRGB.TextMain}
        />
        <rect
          width={8}
          height={2}
          rx={1}
          transform="rotate(135 5.672 6.106)"
          fill={ColorsRGB.TextMain}
        />
      </motion.g>
    </motion.svg>
  );
};
