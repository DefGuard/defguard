import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconArrowGraySmall = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-arrow-gray-small_svg__a">
        <path data-name="Rectangle 2812" fill="#899ca8" opacity={0} d="M0 0h22v22H0z" />
      </clipPath>
    </defs>
    <g clipPath="url(#icon-arrow-gray-small_svg__a)" fill="#899ca8">
      <rect
        data-name="Rectangle 2810"
        width={6}
        height={2}
        rx={1}
        transform="rotate(-135 8.9 3.586)"
      />
      <rect
        data-name="Rectangle 2811"
        width={6}
        height={2}
        rx={1}
        transform="rotate(-45 21.314 -3.07)"
      />
    </g>
  </svg>
);

export default SvgIconArrowGraySmall;
