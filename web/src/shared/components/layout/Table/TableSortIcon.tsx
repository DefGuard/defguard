import { motion, SVGMotionProps } from 'framer-motion';

export const TableSortIcon: React.FC<SVGMotionProps<SVGSVGElement>> = (
  props
) => (
  <motion.svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    {...props}
  >
    <defs>
      <style>
        {
          '.icon-arrow-gray-down_svg__a,.icon-arrow-gray-down_svg__c{fill:#899ca8}.icon-arrow-gray-down_svg__a{opacity:0}'
        }
      </style>
    </defs>
    <g className="icon-arrow-gray-down_svg__b" transform="rotate(-90 11 11)">
      <rect
        className="icon-arrow-gray-down_svg__c"
        width={8}
        height={2}
        rx={1}
        transform="rotate(45 -7.4 14.862)"
      />
      <rect
        className="icon-arrow-gray-down_svg__c"
        width={8}
        height={2}
        rx={1}
        transform="rotate(135 5.672 6.106)"
      />
    </g>
  </motion.svg>
);
