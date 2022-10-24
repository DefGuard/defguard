import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconArrowGrayDown = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-arrow-gray-down_svg__a">
        <path className="icon-arrow-gray-down_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-arrow-gray-down_svg__a,.icon-arrow-gray-down_svg__c{fill:#899ca8}.icon-arrow-gray-down_svg__a{opacity:0}.icon-arrow-gray-down_svg__b{clip-path:url(#icon-arrow-gray-down_svg__a)}\n    '
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
  </svg>
);

export default SvgIconArrowGrayDown;
