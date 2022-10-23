import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconCloseGray = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-close-gray_svg__a">
        <path className="icon-close-gray_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-close-gray_svg__a,.icon-close-gray_svg__c{fill:#cbd3d8}.icon-close-gray_svg__a{opacity:0}.icon-close-gray_svg__b{clip-path:url(#icon-close-gray_svg__a)}\n    '
        }
      </style>
    </defs>
    <g className="icon-close-gray_svg__b" transform="rotate(90 11 11)">
      <rect
        className="icon-close-gray_svg__c"
        width={16}
        height={2}
        rx={1}
        transform="rotate(45 -2.571 9.621)"
      />
      <rect
        className="icon-close-gray_svg__c"
        width={16}
        height={2}
        rx={1}
        transform="rotate(135 7.429 6.621)"
      />
    </g>
  </svg>
);

export default SvgIconCloseGray;
