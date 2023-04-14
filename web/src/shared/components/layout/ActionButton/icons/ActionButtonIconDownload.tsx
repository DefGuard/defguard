import { motion, SVGMotionProps } from 'framer-motion';

import { ColorsRGB } from '../../../../constants';

export const ActionButtonIconDownload = (props: SVGMotionProps<SVGSVGElement>) => {
  return (
    <motion.svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
      <defs>
        <clipPath id="icon-download_svg__a">
          <path
            data-name="Rectangle 2609"
            fill={ColorsRGB.GrayLight}
            opacity={0}
            d="M0 0h22v22H0z"
          />
        </clipPath>
      </defs>
      <g transform="rotate(90 11 11)" clipPath="url(#icon-download_svg__a)">
        <motion.g
          data-name="Group 4637"
          transform="rotate(-90 53.5 280.5)"
          variants={{
            idle: {
              fill: ColorsRGB.GrayLight,
            },
            active: {
              fill: ColorsRGB.White,
            },
          }}
        >
          <rect
            data-name="Rectangle 2606"
            width={6}
            height={2}
            rx={1}
            transform="rotate(90 39.5 278.5)"
          />
          <rect
            data-name="Rectangle 2611"
            width={6}
            height={2}
            rx={1}
            transform="rotate(90 45.5 284.5)"
          />
          <rect
            data-name="Rectangle 2612"
            width={6}
            height={2}
            rx={1}
            transform="rotate(45 -122.381 503.472)"
          />
          <rect
            data-name="Rectangle 2610"
            width={14}
            height={2}
            rx={1}
            transform="translate(316 243)"
          />
          <rect
            data-name="Rectangle 2607"
            width={8}
            height={2}
            rx={1}
            transform="rotate(90 46.5 277.5)"
          />
          <rect
            data-name="Rectangle 2613"
            width={6}
            height={2}
            rx={1}
            transform="rotate(135 114.948 185.516)"
          />
        </motion.g>
      </g>
    </motion.svg>
  );
};
